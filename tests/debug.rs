//! Debug groups, object labels, and the control, insertion and log of debug messages.

#[macro_use]
extern crate safegl;

mod support;

#[cfg(target_arch = "wasm32")]
#[allow(unused_imports)]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[cfg(any(safegl_gl43, safegl_gles30))]
mod debug_output {
    use super::support;

    use std::cell::RefCell;
    use std::rc::Rc;

    use safegl::debug::{
        DebugCallbackBehavior, DebugError, DebugMessage, DebugMessageFilter, MessageType,
        ObjectLabel, Severity, Source,
    };
    use safegl::framebuffer::RenderBuffer;
    use safegl::texture::UncompressedFloatFormat;
    use safegl::{CapabilitiesSource, Surface};

    type Received = Rc<RefCell<Vec<DebugMessage>>>;

    /// A display whose debug callback keeps the messages, and the messages.
    fn display_with_messages() -> (support::WindowDisplay, Received) {
        let received = Received::default();
        let sink = received.clone();
        let display = support::build_display_with_debug(DebugCallbackBehavior::Custom {
            callback: Box::new(move |source, ty, severity, id, _, message: &str| {
                sink.borrow_mut().push(DebugMessage {
                    source,
                    ty,
                    id,
                    severity,
                    message: message.to_owned(),
                })
            }),
            synchronous: true,
        });
        (display, received)
    }

    /// The messages from the application among `received`, without their severity, which the
    /// groups don't fix.
    fn application_messages(received: &Received) -> Vec<(MessageType, u32, String)> {
        received
            .borrow()
            .iter()
            .filter(|message| message.source == Source::Application)
            .map(|message| (message.ty, message.id, message.message.clone()))
            .collect()
    }

    fn marker(id: u32, text: &str) -> DebugMessage {
        DebugMessage {
            source: Source::Application,
            ty: MessageType::Marker,
            id,
            severity: Severity::Notification,
            message: text.to_owned(),
        }
    }

    #[test]
    fn debug_groups_send_messages() {
        let (display, received) = display_with_messages();
        if !display.is_debug_output_supported() {
            return;
        }

        {
            let _outer = display.push_debug_group(1, "outer").unwrap();
            let _inner = display.push_debug_group(2, "inner").unwrap();
        }

        assert_eq!(
            application_messages(&received),
            [
                (MessageType::PushGroup, 1, "outer".to_owned()),
                (MessageType::PushGroup, 2, "inner".to_owned()),
                (MessageType::PopGroup, 2, "inner".to_owned()),
                (MessageType::PopGroup, 1, "outer".to_owned()),
            ]
        );

        display.assert_no_error(None);
    }

    #[test]
    fn debug_group_stack_overflow() {
        let display = support::build_display();
        if !display.is_debug_output_supported() {
            return;
        }
        let max = display
            .get_capabilities()
            .limits
            .max_debug_group_stack_depth
            .unwrap();

        // the stack holds the default group, and is full one group before its maximum depth
        let mut groups = Vec::new();
        for id in 0..max - 2 {
            groups.push(display.push_debug_group(id as u32, "group").unwrap());
        }
        assert_eq!(
            display.push_debug_group(0, "one too many").err(),
            Some(DebugError::StackOverflow)
        );
        display.assert_no_error(None);

        while groups.pop().is_some() {}
        let _group = display.push_debug_group(0, "again").unwrap();

        display.assert_no_error(None);
    }

    #[test]
    fn debug_message_too_long() {
        let display = support::build_display();
        if !display.is_debug_output_supported() {
            return;
        }
        let max = display
            .get_capabilities()
            .limits
            .max_debug_message_length
            .unwrap() as usize;

        let text = "x".repeat(max);
        assert_eq!(
            display.push_debug_group(0, &text).err(),
            Some(DebugError::MessageTooLong)
        );
        assert_eq!(
            display.insert_debug_message(&marker(0, &text)),
            Err(DebugError::MessageTooLong)
        );
        let _group = display.push_debug_group(0, &text[1..]).unwrap();

        display.assert_no_error(None);
    }

    #[test]
    fn inserted_messages_are_filtered() {
        let (display, received) = display_with_messages();
        if !display.is_debug_output_supported() {
            return;
        }

        let filter = DebugMessageFilter {
            source: Some(Source::Application),
            ty: Some(MessageType::Marker),
            ids: &[42],
            ..Default::default()
        };
        display.set_debug_messages_enabled(&filter, false).unwrap();
        display.insert_debug_message(&marker(42, "hidden")).unwrap();
        display.insert_debug_message(&marker(43, "shown")).unwrap();
        display.set_debug_messages_enabled(&filter, true).unwrap();
        display
            .insert_debug_message(&marker(42, "shown again"))
            .unwrap();

        assert_eq!(
            application_messages(&received),
            [
                (MessageType::Marker, 43, "shown".to_owned()),
                (MessageType::Marker, 42, "shown again".to_owned()),
            ]
        );
        assert_eq!(received.borrow()[0].severity, Severity::Notification);

        display.assert_no_error(None);
    }

    #[test]
    fn invalid_filters_and_sources() {
        let display = support::build_display();
        if !display.is_debug_output_supported() {
            return;
        }

        let without_type = DebugMessageFilter {
            source: Some(Source::Application),
            ids: &[1],
            ..Default::default()
        };
        assert_eq!(
            display.set_debug_messages_enabled(&without_type, false),
            Err(DebugError::InvalidFilter)
        );
        let with_severity = DebugMessageFilter {
            source: Some(Source::Application),
            ty: Some(MessageType::Marker),
            severity: Some(Severity::High),
            ids: &[1],
        };
        assert_eq!(
            display.set_debug_messages_enabled(&with_severity, false),
            Err(DebugError::InvalidFilter)
        );
        let api = DebugMessage {
            source: Source::Api,
            ..marker(1, "from the API")
        };
        assert_eq!(
            display.insert_debug_message(&api),
            Err(DebugError::InvalidSource)
        );

        display.assert_no_error(None);
    }

    #[test]
    fn message_log() {
        let display = support::build_display_with_debug(DebugCallbackBehavior::Log);
        if !display.is_debug_output_supported() {
            return;
        }

        // only the messages of the application, which the log returns in order
        display
            .set_debug_messages_enabled(&Default::default(), false)
            .unwrap();
        let application = DebugMessageFilter {
            source: Some(Source::Application),
            ..Default::default()
        };
        display
            .set_debug_messages_enabled(&application, true)
            .unwrap();
        display.debug_message_log(usize::MAX).unwrap();

        for (id, text) in [(1, "first"), (2, "second"), (3, "third")] {
            display.insert_debug_message(&marker(id, text)).unwrap();
        }
        assert_eq!(
            display.debug_message_log(2).unwrap(),
            [marker(1, "first"), marker(2, "second")]
        );
        assert_eq!(display.debug_message_log(10).unwrap(), [marker(3, "third")]);
        assert_eq!(display.debug_message_log(10).unwrap(), []);

        display.assert_no_error(None);
    }

    #[test]
    fn labels_of_objects() {
        let display = support::build_display();
        if !display.is_debug_output_supported() {
            return;
        }

        fn round_trip(object: &dyn ObjectLabel, label: &str) {
            assert_eq!(object.label().unwrap(), "");
            object.set_label(label).unwrap();
            assert_eq!(object.label().unwrap(), label);
            object.set_label("").unwrap();
            assert_eq!(object.label().unwrap(), "");
        }

        let (vertices, indices) = support::build_rectangle_vb_ib(&display);
        round_trip(&*vertices, "vertices");
        round_trip(&*indices, "indices");
        let texture = support::build_renderable_texture(&display);
        round_trip(&*texture, "color target");
        let render_buffer =
            RenderBuffer::new(&display, UncompressedFloatFormat::U8U8U8U8, 4, 4).unwrap();
        round_trip(&*render_buffer, "render buffer");
        let (_, _, program) = support::build_fullscreen_red_pipeline(&display);
        round_trip(&program, "red");
        let query = safegl::draw_parameters::AnySamplesPassedQuery::new(&display, false).unwrap();
        round_trip(&query, "occlusion");

        // OpenGL creates the query when it begins, with the label given before
        query.set_label("occlusion").unwrap();
        let (vb, ib, program) = support::build_fullscreen_red_pipeline(&display);
        let target = support::build_renderable_texture(&display);
        let parameters = safegl::DrawParameters {
            samples_passed_query: Some((&query).into()),
            ..Default::default()
        };
        target
            .as_surface()
            .draw(&vb, &ib, &program, &uniform! {}, &parameters)
            .unwrap();
        assert_eq!(query.label().unwrap(), "occlusion");
        let fence = safegl::SyncFence::new(&display);
        round_trip(&fence, "fence");
        fence.wait();

        let max = display.get_capabilities().limits.max_label_length.unwrap() as usize;
        assert_eq!(
            program.set_label(&"x".repeat(max)),
            Err(DebugError::LabelTooLong)
        );
        program.set_label(&"x".repeat(max - 1)).unwrap();
        assert_eq!(program.label().unwrap().len(), max - 1);

        display.assert_no_error(None);
    }

    #[cfg(any(safegl_gl43, safegl_gles31))]
    #[test]
    fn label_of_a_compute_shader() {
        let display = support::build_display();
        if !display.is_debug_output_supported()
            || !safegl::program::ComputeShader::is_supported(&display)
            || !support::glsl_supported(&display, 430, 310)
        {
            return;
        }
        let version = if support::is_gles(&display) {
            "310 es"
        } else {
            "430"
        };
        let shader = safegl::program::ComputeShader::from_source(
            &display,
            &format!("#version {version}\nlayout(local_size_x = 1) in;\nvoid main() {{}}\n"),
        )
        .unwrap();

        shader.set_label("compute").unwrap();
        assert_eq!(shader.label().unwrap(), "compute");

        display.assert_no_error(None);
    }

    /// Without `GL_KHR_debug`, the objects other than sync fences have labels with
    /// `GL_EXT_debug_label`, and debug groups and inserted messages are the markers of
    /// `GL_EXT_debug_marker`.
    #[test]
    fn labels_and_markers_of_extensions() {
        let display = support::build_display();
        if display.is_debug_output_supported() {
            return;
        }
        let extensions = support::extensions(&display);
        let labels = extensions.gl_ext_debug_label;
        let markers = extensions.gl_ext_debug_marker;

        let (vertices, _) = support::build_rectangle_vb_ib(&display);
        match vertices.set_label("vertices") {
            Ok(()) => {
                assert!(labels);
                assert_eq!(vertices.label().unwrap(), "vertices");
                vertices.set_label("").unwrap();
                assert_eq!(vertices.label().unwrap(), "");
                let (_, _, program) = support::build_fullscreen_red_pipeline(&display);
                program.set_label("red").unwrap();
                assert_eq!(program.label().unwrap(), "red");
            }
            Err(DebugError::NotSupported) => assert!(!labels),
            Err(error) => panic!("{error}"),
        }
        let fence = safegl::SyncFence::new(&display);
        assert_eq!(fence.set_label("fence"), Err(DebugError::NotSupported));
        fence.wait();

        match display.push_debug_group(1, "group") {
            Ok(group) => {
                assert!(markers);
                let event = DebugMessage {
                    source: Source::Application,
                    ty: MessageType::Marker,
                    id: 0,
                    severity: Severity::Notification,
                    message: "event".to_owned(),
                };
                display.insert_debug_message(&event).unwrap();
                drop(group);
            }
            Err(DebugError::NotSupported) => assert!(!markers),
            Err(error) => panic!("{error}"),
        }

        display.assert_no_error(None);
    }
}
