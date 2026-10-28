#[macro_use]
extern crate safegl;

use safegl::buffer::{Buffer, BufferMode, BufferType, Content};

mod support;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

struct Values {
    count: u32,
    values: [u32],
}

implement_buffer_content!(Values);
implement_any_bit_pattern!(Values, count, values);

struct Generic<T> {
    values: [T],
}

implement_buffer_content!(Generic<T>);
implement_any_bit_pattern!(Generic<T>, values);

#[test]
fn sizes() {
    assert_eq!(<Values as Content>::get_elements_size(), 4);
    assert!(!<Values as Content>::is_size_suitable(4));
    assert!(<Values as Content>::is_size_suitable(8));
    assert!(<Values as Content>::is_size_suitable(16));
    assert!(!<Values as Content>::is_size_suitable(10));

    assert!(<Generic<u64> as Content>::is_size_suitable(24));
    assert!(!<Generic<u64> as Content>::is_size_suitable(20));
}

#[test]
fn read_without_opengl() {
    let value = unsafe {
        <Values as Content>::read(16, |value| {
            value.count = 3;
            value.values.copy_from_slice(&[1, 2, 3]);
            Ok::<_, ()>(())
        })
    }
    .unwrap();
    assert_eq!(value.count, 3);
    assert_eq!(&value.values, &[1, 2, 3]);

    let pointer = <Values as Content>::ref_from_ptr(value.to_void_ptr() as *mut (), 16).unwrap();
    assert_eq!(unsafe { &(*pointer).values }, &[1, 2, 3]);
    assert!(<Values as Content>::ref_from_ptr(value.to_void_ptr() as *mut (), 10).is_none());
}

#[test]
fn round_trip_through_a_buffer() {
    let display = support::build_display();

    let mut buffer =
        Buffer::<Values>::empty_unsized(&display, BufferType::ArrayBuffer, 20, BufferMode::Default)
            .unwrap();
    {
        let mut mapping = buffer.map();
        mapping.count = 4;
        mapping.values.copy_from_slice(&[5, 6, 7, 8]);
    }
    let value = buffer.read().unwrap();
    assert_eq!(value.count, 4);
    assert_eq!(&value.values, &[5, 6, 7, 8]);

    let mut generic = Buffer::<Generic<f32>>::empty_unsized(
        &display,
        BufferType::ArrayBuffer,
        8,
        BufferMode::Default,
    )
    .unwrap();
    generic.map().values.copy_from_slice(&[0.5, 1.5]);
    assert_eq!(&generic.read().unwrap().values, &[0.5, 1.5]);

    display.assert_no_error(None);
}

#[test]
fn several_unsized_uniform_buffers() {
    struct Bones {
        matrices: [[[f32; 4]; 4]],
    }
    implement_buffer_content!(Bones);
    implement_any_bit_pattern!(Bones, matrices);

    let display = support::build_display();
    let mut buffers: Vec<_> = [21, 1, 1]
        .iter()
        .map(|&count| {
            safegl::uniforms::UniformBuffer::<Bones>::empty_unsized_dynamic(&display, count * 64)
                .unwrap()
        })
        .collect();

    // every buffer is mapped with its own number of matrices
    for (i, buffer) in buffers.iter_mut().enumerate() {
        let mut mapping = buffer.map();
        for matrix in mapping.matrices.iter_mut() {
            *matrix = [[i as f32; 4]; 4];
        }
    }
    for (i, buffer) in buffers.iter().enumerate() {
        let bones = buffer.read().unwrap();
        assert_eq!(bones.matrices.len(), [21, 1, 1][i]);
        assert!(bones.matrices.iter().all(|m| *m == [[i as f32; 4]; 4]));
    }
    display.assert_no_error(None);
}

#[test]
fn any_bit_pattern_structs() {
    use safegl::buffer::AnyBitPattern;

    fn is_any_bit_pattern<T: ?Sized + AnyBitPattern>() {}

    #[derive(Copy, Clone)]
    #[allow(dead_code)]
    struct Named {
        position: [f32; 3],
        id: u32,
    }
    implement_any_bit_pattern!(Named, position, id);

    #[derive(Copy, Clone)]
    #[allow(dead_code)]
    struct Tuple(f32, (u8, u8), [i16; 2]);
    implement_any_bit_pattern!(Tuple, 0, 1, 2);

    #[derive(Copy, Clone)]
    #[allow(dead_code)]
    struct Pair<T> {
        first: T,
        second: T,
    }
    implement_any_bit_pattern!(Pair<T>, first, second);

    is_any_bit_pattern::<Named>();
    is_any_bit_pattern::<Tuple>();
    is_any_bit_pattern::<Pair<f64>>();
    is_any_bit_pattern::<[Named]>();
    is_any_bit_pattern::<Values>();
    is_any_bit_pattern::<Generic<u16>>();
}

#[test]
fn read_empty_slice_without_opengl() {
    // reading zero elements gives an empty vector, and the empty slice still has a pointer
    let value = unsafe { <[u32] as Content>::read(0, |_| Ok::<_, ()>(())) }.unwrap();
    assert!(value.is_empty());
    assert!(!<[u32] as Content>::to_void_ptr(&value[..]).is_null());
}
