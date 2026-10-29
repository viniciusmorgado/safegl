//! Debug groups, object labels, and the control, insertion and log of debug messages (OpenGL
//! 4.3, OpenGL ES 3.2 and `GL_KHR_debug`). Without them, objects have labels with
//! `GL_EXT_debug_label`, and groups and inserted markers with `GL_EXT_debug_marker`.

use std::cell::RefCell;

use glow::HasContext;

use crate::context::{CommandContext, Context};
use crate::debug::sealed::LabeledObject;
use crate::debug::{DebugError, DebugMessage, DebugMessageFilter, MessageType, Severity, Source};
use crate::gl;
use crate::version::{is_webgl, Api, Version};
use crate::ContextExt;

/// Returns true if the context has the debug output of `GL_KHR_debug`.
pub(crate) fn is_supported(ctxt: &CommandContext<'_>) -> bool {
    !is_webgl()
        && (ctxt.version >= &Version(Api::Gl, 4, 3)
            || ctxt.version >= &Version(Api::GlEs, 3, 2)
            || ctxt.extensions.gl_khr_debug)
}

/// Returns true if the context labels objects: with the debug output, or with
/// `GL_EXT_debug_label`.
fn labels_supported(ctxt: &CommandContext<'_>) -> bool {
    is_supported(ctxt) || (!is_webgl() && ctxt.extensions.gl_ext_debug_label)
}

/// Returns true if the context has debug groups and inserted messages: with the debug output,
/// or as the markers of `GL_EXT_debug_marker`.
fn markers_supported(ctxt: &CommandContext<'_>) -> bool {
    is_supported(ctxt) || (!is_webgl() && ctxt.extensions.gl_ext_debug_marker)
}

/// The object type of `GL_EXT_debug_label` for an identifier of `GL_KHR_debug`; the others are
/// the same.
fn ext_object_type(identifier: gl::types::GLenum) -> gl::types::GLenum {
    match identifier {
        gl::BUFFER => 0x9151,           // GL_BUFFER_OBJECT_EXT
        gl::SHADER => 0x8B48,           // GL_SHADER_OBJECT_EXT
        gl::PROGRAM => 0x8B40,          // GL_PROGRAM_OBJECT_EXT
        gl::VERTEX_ARRAY => 0x9154,     // GL_VERTEX_ARRAY_OBJECT_EXT
        gl::QUERY => 0x9153,            // GL_QUERY_OBJECT_EXT
        gl::PROGRAM_PIPELINE => 0x8A4F, // GL_PROGRAM_PIPELINE_OBJECT_EXT
        other => other,
    }
}

/// Gives the OpenGL object `name` of the namespace `identifier` a label, or removes it, with
/// `GL_KHR_debug` or else `GL_EXT_debug_label`.
pub(crate) unsafe fn apply_label(
    ctxt: &CommandContext<'_>,
    identifier: gl::types::GLenum,
    name: gl::types::GLuint,
    label: Option<&str>,
) {
    if is_supported(ctxt) {
        ctxt.glow.object_label(identifier, name, label);
    } else {
        // a length of 0 means a null-terminated label: a null label removes it
        let (length, pointer) = match label {
            Some(label) => (label.len() as gl::types::GLsizei, label.as_ptr().cast()),
            None => (0, std::ptr::null()),
        };
        ctxt.extra
            .LabelObjectEXT(ext_object_type(identifier), name, length, pointer);
    }
}

/// Returns true if the entry points have the `KHR` suffix, which they have on OpenGL ES below 3.2.
fn khr_suffix(ctxt: &CommandContext<'_>) -> bool {
    ctxt.version.0 == Api::GlEs && ctxt.version < &Version(Api::GlEs, 3, 2)
}

/// Returns true if `text` is too long for a limit that counts a null terminator.
fn too_long(text: &str, limit: Option<gl::types::GLint>) -> bool {
    text.len() as i64 >= limit.unwrap_or(0) as i64
}

/// A debug group, pushed by `Context::push_debug_group` and popped when dropped.
///
/// The debug messages of the commands inside the group are attributed to it, and debugging tools
/// show the commands nested in it. Groups are popped in the order they are dropped: drop the
/// innermost one first.
#[must_use = "the debug group is popped as soon as it is dropped"]
pub struct DebugGroup<'a> {
    context: &'a Context,
    /// The `generation` of the context when the group was pushed.
    generation: u64,
    /// True if the group is a marker of `GL_EXT_debug_marker`.
    marker: bool,
}

impl Drop for DebugGroup<'_> {
    fn drop(&mut self) {
        // a group pushed before the context was rebuilt went away with the previous context
        if self.context.generation.get() != self.generation {
            return;
        }
        let depth = self.context.debug_group_depth.get();
        if depth == 0 {
            return;
        }
        self.context.debug_group_depth.set(depth - 1);

        let ctxt = self.context.make_current();
        unsafe {
            if self.marker {
                ctxt.extra.PopGroupMarkerEXT();
            } else {
                ctxt.glow.pop_debug_group();
            }
        }
    }
}

impl Context {
    /// Returns true if the context has debug groups, object labels and the control, insertion
    /// and log of debug messages: OpenGL 4.3, OpenGL ES 3.2 or `GL_KHR_debug`.
    pub fn is_debug_output_supported(&self) -> bool {
        is_supported(&self.make_current())
    }

    /// Pushes a debug group with an identifier and a message, which is popped when the returned
    /// `DebugGroup` is dropped. Pushing and popping send messages of the types `PushGroup` and
    /// `PopGroup` from `Source::Application` to the debug output.
    ///
    /// With `GL_EXT_debug_marker` but without the debug output, the group is a group
    /// marker that debugging tools show, without an identifier nor messages.
    pub fn push_debug_group(&self, id: u32, message: &str) -> Result<DebugGroup<'_>, DebugError> {
        let ctxt = self.make_current();
        if !is_supported(&ctxt) {
            if !markers_supported(&ctxt) {
                return Err(DebugError::NotSupported);
            }
            let length = message.len() as gl::types::GLsizei;
            unsafe { ctxt.extra.PushGroupMarkerEXT(length, message.as_ptr().cast()) };
            self.debug_group_depth.set(self.debug_group_depth.get() + 1);
            return Ok(DebugGroup {
                context: self,
                generation: self.generation.get(),
                marker: true,
            });
        }
        let limits = &self.capabilities.limits;
        if too_long(message, limits.max_debug_message_length) {
            return Err(DebugError::MessageTooLong);
        }

        // the stack starts with the default group, and `glPushDebugGroup` fails when it holds
        // `GL_MAX_DEBUG_GROUP_STACK_DEPTH` minus one groups
        let depth = self.debug_group_depth.get();
        let max_depth = limits.max_debug_group_stack_depth.unwrap_or(0) as i64;
        if depth as i64 + 2 >= max_depth {
            return Err(DebugError::StackOverflow);
        }

        unsafe {
            ctxt.glow
                .push_debug_group(gl::DEBUG_SOURCE_APPLICATION, id, message)
        };
        self.debug_group_depth.set(depth + 1);

        Ok(DebugGroup {
            context: self,
            generation: self.generation.get(),
            marker: false,
        })
    }

    /// Enables or disables the debug messages that `filter` matches, which the driver then
    /// generates or not. Every message is enabled when the context is created, except those of
    /// low severity and the notifications with `DebugCallbackBehavior::DebugMessageOnError`.
    pub fn set_debug_messages_enabled(
        &self,
        filter: &DebugMessageFilter<'_>,
        enabled: bool,
    ) -> Result<(), DebugError> {
        let ctxt = self.make_current();
        if !is_supported(&ctxt) {
            return Err(DebugError::NotSupported);
        }
        if !filter.ids.is_empty()
            && (filter.source.is_none() || filter.ty.is_none() || filter.severity.is_some())
        {
            return Err(DebugError::InvalidFilter);
        }

        unsafe {
            ctxt.glow.debug_message_control(
                filter.source.map_or(gl::DONT_CARE, |source| source as u32),
                filter.ty.map_or(gl::DONT_CARE, |ty| ty as u32),
                filter
                    .severity
                    .map_or(gl::DONT_CARE, |severity| severity as u32),
                filter.ids,
                enabled,
            )
        };
        Ok(())
    }

    /// Sends a message to the debug output, from `Source::Application` or `Source::ThirdParty`.
    /// It reaches the debug callback or the message log if it is enabled.
    ///
    /// With `GL_EXT_debug_marker` but without the debug output, the text of the
    /// message is inserted as an event marker that debugging tools show.
    pub fn insert_debug_message(&self, message: &DebugMessage) -> Result<(), DebugError> {
        let ctxt = self.make_current();
        if !markers_supported(&ctxt) {
            return Err(DebugError::NotSupported);
        }
        if message.source != Source::Application && message.source != Source::ThirdParty {
            return Err(DebugError::InvalidSource);
        }
        if !is_supported(&ctxt) {
            let text = &message.message;
            let length = text.len() as gl::types::GLsizei;
            unsafe { ctxt.extra.InsertEventMarkerEXT(length, text.as_ptr().cast()) };
            return Ok(());
        }
        if too_long(
            &message.message,
            self.capabilities.limits.max_debug_message_length,
        ) {
            return Err(DebugError::MessageTooLong);
        }

        unsafe {
            ctxt.glow.debug_message_insert(
                message.source as u32,
                message.ty as u32,
                message.id,
                message.severity as u32,
                &message.message,
            )
        };
        Ok(())
    }

    /// Takes up to `max` messages out of the message log of the context, oldest first.
    ///
    /// Messages are only logged when the debug output has no callback: create the context with
    /// `DebugCallbackBehavior::Log`. The log keeps up to `Limits::max_debug_logged_messages`
    /// messages and drops the newer ones when it is full.
    pub fn debug_message_log(&self, max: usize) -> Result<Vec<DebugMessage>, DebugError> {
        let ctxt = self.make_current();
        if !is_supported(&ctxt) {
            return Err(DebugError::NotSupported);
        }

        let mut messages = Vec::new();
        while messages.len() < max {
            // the length of the next message, with its null terminator, or 0 if the log is empty
            let length = unsafe {
                ctxt.glow
                    .get_parameter_i32(gl::DEBUG_NEXT_LOGGED_MESSAGE_LENGTH)
            };
            if length <= 0 {
                break;
            }

            let mut text = vec![0u8; length as usize];
            let (mut source, mut ty, mut id, mut severity, mut written) = (0, 0, 0, 0, 0);
            let received = unsafe {
                let get = if khr_suffix(&ctxt) {
                    gl::Extra::GetDebugMessageLogKHR
                } else {
                    gl::Extra::GetDebugMessageLog
                };
                get(
                    ctxt.extra,
                    1,
                    length,
                    &mut source,
                    &mut ty,
                    &mut id,
                    &mut severity,
                    &mut written,
                    text.as_mut_ptr() as *mut gl::types::GLchar,
                )
            };
            if received == 0 {
                break;
            }

            // a message of another value than those of the specification is dropped
            text.truncate((written.max(1) - 1) as usize);
            if let (Some(source), Some(ty), Some(severity)) = (
                Source::from_glenum(source),
                MessageType::from_glenum(ty),
                Severity::from_glenum(severity),
            ) {
                messages.push(DebugMessage {
                    source,
                    ty,
                    id,
                    severity,
                    message: String::from_utf8_lossy(&text).into_owned(),
                });
            }
        }
        Ok(messages)
    }
}

/// Gives an object a label, or removes it if `label` is empty.
///
/// The label of an object that OpenGL hasn't created yet is kept in `pending` until it is.
pub(crate) fn set_object_label(
    context: &Context,
    object: LabeledObject,
    pending: Option<&RefCell<Option<String>>>,
    label: &str,
) -> Result<(), DebugError> {
    let ctxt = context.make_current();
    let khr = is_supported(&ctxt);
    // `GL_EXT_debug_label` has no limit, and no labels of sync objects
    let ext = !khr && labels_supported(&ctxt) && !matches!(object, LabeledObject::Sync(_));
    if !khr && !ext {
        return Err(DebugError::NotSupported);
    }
    if khr && too_long(label, context.capabilities.limits.max_label_length) {
        return Err(DebugError::LabelTooLong);
    }

    let label = if label.is_empty() { None } else { Some(label) };
    if let Some(pending) = pending {
        *pending.borrow_mut() = label.map(str::to_owned);
        return Ok(());
    }
    unsafe {
        match object {
            LabeledObject::Object(identifier, name) => {
                apply_label(&ctxt, identifier, name, label)
            }
            LabeledObject::Sync(sync) => ctxt.glow.object_ptr_label(sync, label),
        }
    }
    Ok(())
}

/// Returns the label of an object, empty if it has none.
pub(crate) fn object_label(
    context: &Context,
    object: LabeledObject,
    pending: Option<&RefCell<Option<String>>>,
) -> Result<String, DebugError> {
    let ctxt = context.make_current();
    let khr = is_supported(&ctxt);
    let ext = !khr && labels_supported(&ctxt) && !matches!(object, LabeledObject::Sync(_));
    if !khr && !ext {
        return Err(DebugError::NotSupported);
    }
    if let Some(pending) = pending {
        return Ok(pending.borrow().clone().unwrap_or_default());
    }
    if let (true, LabeledObject::Object(identifier, name)) = (ext, &object) {
        // the length of the label, then the label and its null terminator
        let ty = ext_object_type(*identifier);
        let mut length = 0;
        let mut label = unsafe {
            ctxt.extra
                .GetObjectLabelEXT(ty, *name, 0, &mut length, std::ptr::null_mut());
            vec![0u8; length.max(0) as usize + 1]
        };
        unsafe {
            let size = label.len() as gl::types::GLsizei;
            ctxt.extra
                .GetObjectLabelEXT(ty, *name, size, &mut length, label.as_mut_ptr().cast());
        }
        label.truncate(length.max(0) as usize);
        return Ok(String::from_utf8_lossy(&label).into_owned());
    }

    let size = context
        .capabilities
        .limits
        .max_label_length
        .unwrap_or(0)
        .max(1);
    let mut label = vec![0u8; size as usize];
    let mut length = 0;
    let khr = khr_suffix(&ctxt);
    unsafe {
        let buffer = label.as_mut_ptr() as *mut gl::types::GLchar;
        match object {
            LabeledObject::Object(identifier, name) if khr => {
                ctxt.extra
                    .GetObjectLabelKHR(identifier, name, size, &mut length, buffer)
            }
            LabeledObject::Object(identifier, name) => {
                ctxt.extra
                    .GetObjectLabel(identifier, name, size, &mut length, buffer)
            }
            LabeledObject::Sync(sync) => {
                let sync = sync.0 as *const _;
                if khr {
                    ctxt.extra
                        .GetObjectPtrLabelKHR(sync, size, &mut length, buffer)
                } else {
                    ctxt.extra
                        .GetObjectPtrLabel(sync, size, &mut length, buffer)
                }
            }
        }
    }
    label.truncate(length.max(0) as usize);
    Ok(String::from_utf8_lossy(&label).into_owned())
}
