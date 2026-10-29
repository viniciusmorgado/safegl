//! Memory and semaphores shared with Vulkan through file descriptors: Vulkan writes memory that
//! SafeGL reads, and SafeGL signals a semaphore that Vulkan waits for.

#![cfg(all(not(target_arch = "wasm32"), target_os = "linux"))]

#[macro_use]
extern crate safegl;

use std::os::fd::{FromRawFd, OwnedFd};

use ash::vk;
use safegl::buffer::BufferType;
use safegl::interop::{
    device_uuids, is_memory_import_supported, is_semaphore_import_supported, InteropError,
    MemoryObject, Semaphore,
};
use safegl::texture::UncompressedFloatFormat;
use safegl::Surface;

mod support;

/// A Vulkan device that shares its memory with the context of the tests.
struct Vulkan {
    _entry: ash::Entry,
    instance: ash::Instance,
    device: ash::Device,
    physical: vk::PhysicalDevice,
    queue: vk::Queue,
}

impl Drop for Vulkan {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_device(None);
            self.instance.destroy_instance(None);
        }
    }
}

/// The Vulkan device whose UUIDs are `uuids`, with the extensions that export memory and
/// semaphores, if the system has one.
fn vulkan(uuids: ([u8; 16], [u8; 16])) -> Option<Vulkan> {
    let entry = unsafe { ash::Entry::load() }.ok()?;
    let application = vk::ApplicationInfo::default().api_version(vk::API_VERSION_1_1);
    let instance_info = vk::InstanceCreateInfo::default().application_info(&application);
    let instance = unsafe { entry.create_instance(&instance_info, None) }.ok()?;
    let physical = unsafe { instance.enumerate_physical_devices() }
        .ok()?
        .into_iter()
        .find(|&physical| {
            let mut ids = vk::PhysicalDeviceIDProperties::default();
            let mut properties = vk::PhysicalDeviceProperties2::default().push_next(&mut ids);
            unsafe { instance.get_physical_device_properties2(physical, &mut properties) };
            (ids.device_uuid, ids.driver_uuid) == uuids
        });
    let Some(physical) = physical else {
        unsafe { instance.destroy_instance(None) };
        return None;
    };
    let priorities = [1.0];
    let queues = [vk::DeviceQueueCreateInfo::default()
        .queue_family_index(0)
        .queue_priorities(&priorities)];
    let extensions = [
        ash::khr::external_memory_fd::NAME.as_ptr(),
        ash::khr::external_semaphore_fd::NAME.as_ptr(),
    ];
    let device_info = vk::DeviceCreateInfo::default()
        .queue_create_infos(&queues)
        .enabled_extension_names(&extensions);
    let device = unsafe { instance.create_device(physical, &device_info, None) }.ok()?;
    let queue = unsafe { device.get_device_queue(0, 0) };
    Some(Vulkan {
        _entry: entry,
        instance,
        device,
        physical,
        queue,
    })
}

impl Vulkan {
    /// Allocates `size` bytes of memory visible to the CPU that can be exported, with `data`
    /// written at its start, and returns it with its file descriptor.
    fn memory(&self, size: u64, data: &[u32]) -> (vk::DeviceMemory, OwnedFd) {
        let properties =
            unsafe { self.instance.get_physical_device_memory_properties(self.physical) };
        let wanted = vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT;
        let index = properties.memory_types[..properties.memory_type_count as usize]
            .iter()
            .position(|memory_type| memory_type.property_flags.contains(wanted))
            .expect("no memory visible to the CPU") as u32;
        let mut export = vk::ExportMemoryAllocateInfo::default()
            .handle_types(vk::ExternalMemoryHandleTypeFlags::OPAQUE_FD);
        let allocation = vk::MemoryAllocateInfo::default()
            .allocation_size(size)
            .memory_type_index(index)
            .push_next(&mut export);
        unsafe {
            let memory = self.device.allocate_memory(&allocation, None).unwrap();
            let pointer = self
                .device
                .map_memory(memory, 0, size, vk::MemoryMapFlags::empty())
                .unwrap() as *mut u32;
            std::ptr::copy_nonoverlapping(data.as_ptr(), pointer, data.len());
            self.device.unmap_memory(memory);
            let fds = ash::khr::external_memory_fd::Device::new(&self.instance, &self.device);
            let info = vk::MemoryGetFdInfoKHR::default()
                .memory(memory)
                .handle_type(vk::ExternalMemoryHandleTypeFlags::OPAQUE_FD);
            let fd = fds.get_memory_fd(&info).unwrap();
            (memory, OwnedFd::from_raw_fd(fd))
        }
    }

    /// Creates a semaphore that can be exported, and returns it with its file descriptor.
    fn semaphore(&self) -> (vk::Semaphore, OwnedFd) {
        let mut export = vk::ExportSemaphoreCreateInfo::default()
            .handle_types(vk::ExternalSemaphoreHandleTypeFlags::OPAQUE_FD);
        let info = vk::SemaphoreCreateInfo::default().push_next(&mut export);
        unsafe {
            let semaphore = self.device.create_semaphore(&info, None).unwrap();
            let fds = ash::khr::external_semaphore_fd::Device::new(&self.instance, &self.device);
            let info = vk::SemaphoreGetFdInfoKHR::default()
                .semaphore(semaphore)
                .handle_type(vk::ExternalSemaphoreHandleTypeFlags::OPAQUE_FD);
            let fd = fds.get_semaphore_fd(&info).unwrap();
            (semaphore, OwnedFd::from_raw_fd(fd))
        }
    }
}

/// Vulkan writes memory, and SafeGL reads it through a buffer, and draws into a texture created
/// in it.
#[test]
fn memory_shared_with_vulkan() {
    let display = support::build_display();
    if !is_memory_import_supported(&display) {
        let (fd, _) = std::io::pipe().unwrap();
        let result = unsafe { MemoryObject::import_fd(&display, fd.into(), 256, false) };
        assert!(matches!(result, Err(InteropError::NotSupported)));
        return;
    }
    let Some(vulkan) = device_uuids(&display).and_then(vulkan) else {
        return;
    };
    let data: Vec<u32> = (0..64).map(|i| i * 3 + 1).collect();
    let size = 1 << 20;
    let (memory, fd) = vulkan.memory(size, &data);
    let object = unsafe { MemoryObject::import_fd(&display, fd, size, false) }.unwrap();
    assert_eq!(object.size(), size);

    let buffer = object
        .buffer::<u32>(BufferType::ArrayBuffer, data.len(), 0)
        .unwrap();
    assert_eq!(buffer.read().unwrap(), data);
    assert!(matches!(
        object.buffer::<u32>(BufferType::ArrayBuffer, size as usize, 0),
        Err(InteropError::DoesNotFit)
    ));

    // a texture in the memory after the buffer, which SafeGL draws into
    let texture = object
        .texture_2d(UncompressedFloatFormat::U8U8U8U8, 16, 16, 1, 65536)
        .unwrap();
    texture.as_surface().clear_color(0.0, 1.0, 0.0, 1.0);
    let pixels: Vec<Vec<(u8, u8, u8, u8)>> = texture.read().unwrap();
    assert!(pixels.iter().flatten().all(|&pixel| pixel == (0, 255, 0, 255)));
    drop((buffer, texture, object));
    display.assert_no_error(None);
    unsafe { vulkan.device.free_memory(memory, None) };
}

/// SafeGL signals a semaphore that Vulkan waits for.
#[test]
fn semaphore_shared_with_vulkan() {
    let display = support::build_display();
    if !is_semaphore_import_supported(&display) {
        return;
    }
    let Some(vulkan) = device_uuids(&display).and_then(vulkan) else {
        return;
    };
    let (vk_semaphore, fd) = vulkan.semaphore();
    let semaphore = unsafe { Semaphore::import_fd(&display, fd) }.unwrap();
    let texture = safegl::Texture2d::empty(&display, 4, 4).unwrap();
    texture.as_surface().clear_color(1.0, 0.0, 0.0, 1.0);
    semaphore
        .signal(&[], &[(&texture, safegl::interop::ImageLayout::General)])
        .unwrap();

    let waits = [vk_semaphore];
    let stages = [vk::PipelineStageFlags::ALL_COMMANDS];
    let submit = vk::SubmitInfo::default()
        .wait_semaphores(&waits)
        .wait_dst_stage_mask(&stages);
    unsafe {
        vulkan
            .device
            .queue_submit(vulkan.queue, &[submit], vk::Fence::null())
            .unwrap();
        vulkan.device.queue_wait_idle(vulkan.queue).unwrap();
        vulkan.device.destroy_semaphore(vk_semaphore, None);
    }
    display.assert_no_error(None);
}
