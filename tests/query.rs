#[macro_use]
extern crate safegl;

use safegl::Surface;

mod support;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

#[cfg(safegl_desktop)]
#[test]
fn query_sequence() {
    let display = support::build_display();

    let query = match safegl::draw_parameters::SamplesPassedQuery::new(&display) {
        Err(safegl::draw_parameters::QueryCreationError::NotSupported) => return,
        Ok(q) => q,
    };

    let (vb, ib, program) = support::build_fullscreen_red_pipeline(&display);

    let texture = support::build_renderable_texture(&display);
    texture.as_surface().clear_color(0.0, 0.0, 0.0, 0.0);

    for _ in 0..3 {
        let params = safegl::DrawParameters {
            samples_passed_query: Some((&query).into()),
            ..Default::default()
        };

        texture
            .as_surface()
            .draw(&vb, &ib, &program, &safegl::uniforms::EmptyUniforms, &params)
            .unwrap();
    }

    let result = query.get().unwrap();
    assert!(result == 3 * 1024 * 1024); // 3 * texture dimensions

    display.assert_no_error(None);
}

#[cfg(safegl_desktop)]
#[test]
fn samples_passed() {
    let display = support::build_display();

    let query = match safegl::draw_parameters::SamplesPassedQuery::new(&display) {
        Err(safegl::draw_parameters::QueryCreationError::NotSupported) => return,
        Ok(q) => q,
    };

    let (vb, ib, program) = support::build_fullscreen_red_pipeline(&display);

    let texture = support::build_renderable_texture(&display);
    texture.as_surface().clear_color(0.0, 0.0, 0.0, 0.0);

    {
        let params = safegl::DrawParameters {
            samples_passed_query: Some((&query).into()),
            ..Default::default()
        };

        texture
            .as_surface()
            .draw(&vb, &ib, &program, &safegl::uniforms::EmptyUniforms, &params)
            .unwrap();
    }

    let result = query.get().unwrap();
    assert!(result == 1024 * 1024); // texture dimensions

    display.assert_no_error(None);
}

support::async_test! {
    fn any_samples_passed() {
        let display = support::build_display();

        let query = match safegl::draw_parameters::AnySamplesPassedQuery::new(&display, false) {
            Err(safegl::draw_parameters::QueryCreationError::NotSupported) => return,
            Ok(q) => q,
        };

        let (vb, ib, program) = support::build_fullscreen_red_pipeline(&display);

        let texture = support::build_renderable_texture(&display);
        texture.as_surface().clear_color(0.0, 0.0, 0.0, 0.0);

        {
            let params = safegl::DrawParameters {
                samples_passed_query: Some((&query).into()),
                ..Default::default()
            };

            texture
                .as_surface()
                .draw(&vb, &ib, &program, &safegl::uniforms::EmptyUniforms, &params)
                .unwrap();
        }

        // WebGL gives the result in a later task of the event loop
        support::until(|| query.is_ready()).await;
        assert!(query.get().unwrap());

        let query2 = safegl::draw_parameters::AnySamplesPassedQuery::new(&display, false).unwrap();
        assert!(!query2.get().unwrap());

        display.assert_no_error(None);
    }
}

support::async_test! {
    fn any_samples_passed_conservative() {
        let display = support::build_display();

        let query = match safegl::draw_parameters::AnySamplesPassedQuery::new(&display, true) {
            Err(safegl::draw_parameters::QueryCreationError::NotSupported) => return,
            Ok(q) => q,
        };

        let (vb, ib, program) = support::build_fullscreen_red_pipeline(&display);

        let texture = support::build_renderable_texture(&display);
        texture.as_surface().clear_color(0.0, 0.0, 0.0, 0.0);

        {
            let params = safegl::DrawParameters {
                samples_passed_query: Some((&query).into()),
                ..Default::default()
            };

            texture
                .as_surface()
                .draw(&vb, &ib, &program, &safegl::uniforms::EmptyUniforms, &params)
                .unwrap();
        }

        // WebGL gives the result in a later task of the event loop
        support::until(|| query.is_ready()).await;
        assert!(query.get().unwrap());

        let query2 = safegl::draw_parameters::AnySamplesPassedQuery::new(&display, true).unwrap();
        assert!(!query2.get().unwrap());

        display.assert_no_error(None);
    }
}

#[cfg(safegl_desktop)]
#[test]
fn time_elapsed() {
    let display = support::build_display();

    let query = match safegl::draw_parameters::TimeElapsedQuery::new(&display) {
        Err(safegl::draw_parameters::QueryCreationError::NotSupported) => return,
        Ok(q) => q,
    };

    let (vb, ib, program) = support::build_fullscreen_red_pipeline(&display);

    let texture = support::build_renderable_texture(&display);
    texture.as_surface().clear_color(0.0, 0.0, 0.0, 0.0);

    {
        let params = safegl::DrawParameters {
            time_elapsed_query: Some(&query),
            ..Default::default()
        };

        texture
            .as_surface()
            .draw(&vb, &ib, &program, &safegl::uniforms::EmptyUniforms, &params)
            .unwrap();
    }

    let result = query.get().unwrap();
    assert!(result >= 1);

    display.assert_no_error(None);
}

#[cfg(any(safegl_desktop, safegl_gles32))]
#[test]
fn primitives_generated() {
    let display = support::build_display();

    let query = match safegl::draw_parameters::PrimitivesGeneratedQuery::new(&display) {
        Err(safegl::draw_parameters::QueryCreationError::NotSupported) => return,
        Ok(q) => q,
    };

    let (vb, ib, program) = support::build_fullscreen_red_pipeline(&display);

    let texture = support::build_renderable_texture(&display);
    texture.as_surface().clear_color(0.0, 0.0, 0.0, 0.0);

    {
        let params = safegl::DrawParameters {
            primitives_generated_queries: &[&query],
            ..Default::default()
        };

        texture
            .as_surface()
            .draw(&vb, &ib, &program, &safegl::uniforms::EmptyUniforms, &params)
            .unwrap();
    }

    let result = query.get().unwrap();
    assert_eq!(result, 2);

    display.assert_no_error(None);
}

// the transform feedback query is tested with transform feedback, in `vertex_buffer.rs`

#[cfg(safegl_desktop)]
#[test]
fn conditional_render_nodraw() {
    //! a query that was never used counts no samples, so nothing is drawn
    let display = support::build_display();

    let query = match safegl::draw_parameters::AnySamplesPassedQuery::new(&display, false) {
        Err(safegl::draw_parameters::QueryCreationError::NotSupported) => return,
        Ok(q) => q,
    };

    let (vb, ib, program) = support::build_fullscreen_red_pipeline(&display);

    let texture = support::build_renderable_texture(&display);
    texture.as_surface().clear_color(0.0, 0.0, 0.0, 0.0);

    {
        let params = safegl::DrawParameters {
            condition: Some(safegl::draw_parameters::ConditionalRendering {
                query: (&query).into(),
                wait: true,
                per_region: false,
                #[cfg(safegl_gl45)]
                inverted: false,
            }),
            ..Default::default()
        };

        texture
            .as_surface()
            .draw(&vb, &ib, &program, &safegl::uniforms::EmptyUniforms, &params)
            .unwrap();
    }

    let data: Vec<Vec<(u8, u8, u8, u8)>> = texture.read().unwrap();

    for row in data.iter() {
        for pixel in row.iter() {
            assert_eq!(pixel, &(0, 0, 0, 0));
        }
    }

    display.assert_no_error(None);
}

#[cfg(safegl_desktop)]
#[test]
fn conditional_render_simultaneous_query() {
    //! we try to draw with a query and simultaneously use conditional
    //! rendering with the same query

    let display = support::build_display();

    let query = match safegl::draw_parameters::AnySamplesPassedQuery::new(&display, false) {
        Err(safegl::draw_parameters::QueryCreationError::NotSupported) => return,
        Ok(q) => q,
    };

    let (vb, ib, program) = support::build_fullscreen_red_pipeline(&display);

    let texture = support::build_renderable_texture(&display);
    texture.as_surface().clear_color(0.0, 0.0, 0.0, 0.0);

    let params = safegl::DrawParameters {
        samples_passed_query: Some((&query).into()),
        condition: Some(safegl::draw_parameters::ConditionalRendering {
            query: (&query).into(),
            wait: true,
            per_region: false,
            #[cfg(safegl_gl45)]
            inverted: false,
        }),
        ..Default::default()
    };

    let res =
        texture
            .as_surface()
            .draw(&vb, &ib, &program, &safegl::uniforms::EmptyUniforms, &params);

    match res {
        Err(safegl::DrawError::WrongQueryOperation) => (),
        _ => panic!(),
    };

    display.assert_no_error(None);
}

#[cfg(safegl_gl44)]
#[test]
fn query_to_buffer() {
    let display = support::build_display();

    let query = match safegl::draw_parameters::SamplesPassedQuery::new(&display) {
        Err(safegl::draw_parameters::QueryCreationError::NotSupported) => return,
        Ok(q) => q,
    };

    let (vb, ib, program) = support::build_fullscreen_red_pipeline(&display);

    let texture = support::build_renderable_texture(&display);
    texture.as_surface().clear_color(0.0, 0.0, 0.0, 0.0);

    {
        let params = safegl::DrawParameters {
            samples_passed_query: Some((&query).into()),
            ..Default::default()
        };

        texture
            .as_surface()
            .draw(&vb, &ib, &program, &safegl::uniforms::EmptyUniforms, &params)
            .unwrap();
    }

    let mut buffer = safegl::buffer::BufferView::empty(
        &display,
        safegl::buffer::BufferType::ArrayBuffer,
        safegl::buffer::BufferMode::Default,
    )
    .unwrap();
    if query.to_buffer_u32(buffer.as_slice()).is_err() {
        return;
    }

    let mapping = buffer.map();
    assert!(*mapping == 1024 * 1024); // texture dimensions

    display.assert_no_error(None);
}

#[cfg(safegl_desktop)]
#[test]
fn conditional_render_after_query() {
    //! drawing is conditioned by what an earlier draw wrote
    let display = support::build_display();

    let (vb, ib, program) = support::build_fullscreen_red_pipeline(&display);

    for (draw_first, expected) in [(true, (255, 0, 0, 255)), (false, (0, 0, 0, 0))] {
        let query = match safegl::draw_parameters::AnySamplesPassedQuery::new(&display, false) {
            Err(safegl::draw_parameters::QueryCreationError::NotSupported) => return,
            Ok(q) => q,
        };

        let texture = support::build_renderable_texture(&display);
        texture.as_surface().clear_color(0.0, 0.0, 0.0, 0.0);

        // counts the samples of a draw that is entirely clipped away, or of a visible one
        let counted = safegl::DrawParameters {
            samples_passed_query: Some((&query).into()),
            viewport: if draw_first {
                None
            } else {
                Some(safegl::Rect {
                    left: 0,
                    bottom: 0,
                    width: 0,
                    height: 0,
                })
            },
            color_mask: (false, false, false, false),
            ..Default::default()
        };
        texture
            .as_surface()
            .draw(&vb, &ib, &program, &safegl::uniforms::EmptyUniforms, &counted)
            .unwrap();

        let conditioned = safegl::DrawParameters {
            condition: Some(safegl::draw_parameters::ConditionalRendering {
                query: (&query).into(),
                wait: true,
                per_region: false,
                #[cfg(safegl_gl45)]
                inverted: false,
            }),
            ..Default::default()
        };
        match texture.as_surface().draw(
            &vb,
            &ib,
            &program,
            &safegl::uniforms::EmptyUniforms,
            &conditioned,
        ) {
            Err(safegl::DrawError::ConditionalRenderingNotSupported) => return,
            result => result.unwrap(),
        }

        let data: Vec<Vec<(u8, u8, u8, u8)>> = texture.read().unwrap();
        assert!(data.iter().flatten().all(|&p| p == expected));
    }

    display.assert_no_error(None);
}

#[cfg(safegl_desktop)]
#[test]
fn time_elapsed_is_64_bits() {
    let display = support::build_display();

    let query = match safegl::draw_parameters::TimeElapsedQuery::new(&display) {
        Err(safegl::draw_parameters::QueryCreationError::NotSupported) => return,
        Ok(q) => q,
    };
    // never used: its result is 0
    let elapsed: u64 = query.get().unwrap();
    assert_eq!(elapsed, 0);

    display.assert_no_error(None);
}

#[test]
fn timestamp_query_dropped_without_reading() {
    let display = support::build_display();

    for _ in 0..16 {
        drop(safegl::debug::TimestampQuery::new(&display));
    }

    display.assert_no_error(None);
}

#[cfg(safegl_gl44)]
#[test]
fn unused_query_to_buffer() {
    let display = support::build_display();

    let query = match safegl::draw_parameters::SamplesPassedQuery::new(&display) {
        Err(safegl::draw_parameters::QueryCreationError::NotSupported) => return,
        Ok(q) => q,
    };

    let mut buffer = safegl::buffer::Buffer::new(
        &display,
        &7u32,
        safegl::buffer::BufferType::ArrayBuffer,
        safegl::buffer::BufferMode::Default,
    )
    .unwrap();
    if query.to_buffer_u32(buffer.as_slice()).is_err() {
        return;
    }

    assert_eq!(*buffer.map(), 0);

    display.assert_no_error(None);
}

/// Draws the fullscreen red quad on a 1024x1024 texture with `parameters`.
#[cfg(safegl_gl44)]
fn draw_quad(display: &support::WindowDisplay, parameters: &safegl::DrawParameters<'_>) {
    let (vb, ib, program) = support::build_fullscreen_red_pipeline(display);
    let texture = support::build_renderable_texture(display);
    texture.as_surface().clear_color(0.0, 0.0, 0.0, 0.0);
    texture
        .as_surface()
        .draw(&vb, &ib, &program, &safegl::uniforms::EmptyUniforms, parameters)
        .unwrap();
}

/// An empty buffer of one value.
#[cfg(safegl_gl44)]
fn value_buffer<T>(display: &support::WindowDisplay) -> safegl::buffer::Buffer<T>
where
    T: safegl::buffer::Content + safegl::buffer::AnyBitPattern + Copy + 'static,
{
    safegl::buffer::Buffer::empty(
        display,
        safegl::buffer::BufferType::ArrayBuffer,
        safegl::buffer::BufferMode::Default,
    )
    .unwrap()
}

#[cfg(safegl_gl44)]
#[test]
fn query_to_buffers_of_each_type() {
    use safegl::draw_parameters::QueryBufferMode;

    let display = support::build_display();
    let query = match safegl::draw_parameters::SamplesPassedQuery::new(&display) {
        Err(safegl::draw_parameters::QueryCreationError::NotSupported) => return,
        Ok(q) => q,
    };
    draw_quad(
        &display,
        &safegl::DrawParameters {
            samples_passed_query: Some((&query).into()),
            ..Default::default()
        },
    );

    let (u32s, i32s, u64s, i64s) = (
        value_buffer::<u32>(&display),
        value_buffer::<i32>(&display),
        value_buffer::<u64>(&display),
        value_buffer::<i64>(&display),
    );
    if query
        .to_buffer(u32s.as_slice(), QueryBufferMode::Result)
        .is_err()
    {
        return;
    }
    query
        .to_buffer(i32s.as_slice(), QueryBufferMode::Result)
        .unwrap();
    query
        .to_buffer(u64s.as_slice(), QueryBufferMode::Result)
        .unwrap();
    query
        .to_buffer(i64s.as_slice(), QueryBufferMode::Result)
        .unwrap();

    let samples: u64 = 1024 * 1024;
    assert_eq!(u32s.read().unwrap(), samples as u32);
    assert_eq!(i32s.read().unwrap(), samples as i32);
    assert_eq!(u64s.read().unwrap(), samples);
    assert_eq!(i64s.read().unwrap(), samples as i64);
    assert_eq!(query.get().unwrap(), samples);

    display.assert_no_error(None);
}

#[cfg(safegl_gl44)]
#[test]
fn query_availability_and_results_without_waiting() {
    use safegl::draw_parameters::QueryBufferMode;

    let display = support::build_display();
    let unused = match safegl::draw_parameters::SamplesPassedQuery::new(&display) {
        Err(safegl::draw_parameters::QueryCreationError::NotSupported) => return,
        Ok(q) => q,
    };
    let available = value_buffer::<u32>(&display);
    let result = value_buffer::<u32>(&display);
    available.write(&7);
    result.write(&7);
    if unused
        .to_buffer(available.as_slice(), QueryBufferMode::Availability)
        .is_err()
    {
        return;
    }
    // a query that never began has the result 0, available at once
    unused
        .to_buffer(result.as_slice(), QueryBufferMode::ResultNoWait)
        .unwrap();
    assert_eq!(available.read().unwrap(), 1);
    assert_eq!(result.read().unwrap(), 0);

    let query = safegl::draw_parameters::SamplesPassedQuery::new(&display).unwrap();
    draw_quad(
        &display,
        &safegl::DrawParameters {
            samples_passed_query: Some((&query).into()),
            ..Default::default()
        },
    );
    while !query.is_ready() {}
    query
        .to_buffer(available.as_slice(), QueryBufferMode::Availability)
        .unwrap();
    query
        .to_buffer(result.as_slice(), QueryBufferMode::ResultNoWait)
        .unwrap();
    assert_eq!(available.read().unwrap(), 1);
    assert_eq!(result.read().unwrap(), 1024 * 1024);

    display.assert_no_error(None);
}

#[cfg(safegl_gl44)]
#[test]
fn timestamps_to_buffers() {
    use safegl::draw_parameters::QueryBufferMode;

    let display = support::build_display();
    let first = match safegl::debug::TimestampQuery::new(&display) {
        Some(query) => query,
        None => return,
    };
    draw_quad(&display, &Default::default());
    let second = safegl::debug::TimestampQuery::new(&display).unwrap();

    let times = safegl::buffer::Buffer::<[u64]>::empty_array(
        &display,
        safegl::buffer::BufferType::ArrayBuffer,
        2,
        safegl::buffer::BufferMode::Default,
    )
    .unwrap();
    if first
        .to_buffer(times.as_slice().get(0).unwrap(), QueryBufferMode::Result)
        .is_err()
    {
        return;
    }
    second
        .to_buffer(times.as_slice().get(1).unwrap(), QueryBufferMode::Result)
        .unwrap();

    let times = times.read().unwrap();
    assert!(times[0] > 0);
    assert!(times[0] <= times[1]);
    assert_eq!(times[1], second.get().unwrap());

    display.assert_no_error(None);
}

#[cfg(safegl_gl44)]
#[test]
fn query_to_a_buffer_of_another_context() {
    use safegl::draw_parameters::{QueryBufferMode, ToBufferError};

    let display = support::build_display();
    let other = support::build_display();
    let query = match safegl::draw_parameters::SamplesPassedQuery::new(&display) {
        Err(safegl::draw_parameters::QueryCreationError::NotSupported) => return,
        Ok(q) => q,
    };
    let buffer = value_buffer::<u32>(&other);

    match query.to_buffer(buffer.as_slice(), QueryBufferMode::Result) {
        Err(ToBufferError::NotSupported) => (),
        result => assert_eq!(result, Err(ToBufferError::ContextMismatch)),
    }

    display.assert_no_error(None);
    other.assert_no_error(None);
}

#[cfg(safegl_gl45)]
#[test]
fn inverted_conditional_render() {
    //! a query that was never used counts no samples, so the inverted condition draws
    let display = support::build_display();

    let query = match safegl::draw_parameters::AnySamplesPassedQuery::new(&display, false) {
        Err(safegl::draw_parameters::QueryCreationError::NotSupported) => return,
        Ok(q) => q,
    };
    let (vb, ib, program) = support::build_fullscreen_red_pipeline(&display);
    let texture = support::build_renderable_texture(&display);
    texture.as_surface().clear_color(0.0, 0.0, 0.0, 0.0);

    let params = safegl::DrawParameters {
        condition: Some(safegl::draw_parameters::ConditionalRendering {
            query: (&query).into(),
            wait: true,
            per_region: false,
            inverted: true,
        }),
        ..Default::default()
    };
    match texture
        .as_surface()
        .draw(&vb, &ib, &program, &safegl::uniforms::EmptyUniforms, &params)
    {
        Ok(()) => (),
        Err(safegl::DrawError::ConditionalRenderingInvertedNotSupported) => {
            use safegl::CapabilitiesSource;
            assert!(
                !(display.get_opengl_version() >= &safegl::Version(safegl::Api::Gl, 4, 5)
                    || display.get_extensions().gl_arb_conditional_render_inverted)
            );
            return;
        }
        Err(error) => panic!("{error}"),
    }

    let data: Vec<Vec<(u8, u8, u8, u8)>> = texture.read().unwrap();
    assert_eq!(data[0][0], (255, 0, 0, 255));

    display.assert_no_error(None);
}

#[cfg(safegl_desktop)]
#[test]
fn query_that_never_began_is_ready() {
    let display = support::build_display();
    let query = match safegl::draw_parameters::SamplesPassedQuery::new(&display) {
        Err(safegl::draw_parameters::QueryCreationError::NotSupported) => return,
        Ok(q) => q,
    };

    // its result is 0, at once
    assert!(query.is_ready());
    assert_eq!(query.get().unwrap(), 0);

    display.assert_no_error(None);
}

/// Primitives queries count one vertex stream each, and two of the same stream can't count a
/// draw together.
#[cfg(safegl_desktop)]
#[test]
fn primitives_queries_of_streams() {
    use safegl::draw_parameters::PrimitivesGeneratedQuery;
    use safegl::CapabilitiesSource;

    let display = support::build_display();
    if support::is_gles(&display) {
        assert!(matches!(
            PrimitivesGeneratedQuery::with_stream(&display, 1),
            Err(safegl::draw_parameters::QueryCreationError::NotSupported)
        ));
        return;
    }
    let max = display.get_capabilities().limits.max_vertex_streams.unwrap() as u32;
    assert!(matches!(
        PrimitivesGeneratedQuery::with_stream(&display, max),
        Err(safegl::draw_parameters::QueryCreationError::NotSupported)
    ));

    let (vb, ib, program) = support::build_fullscreen_red_pipeline(&display);
    let texture = support::build_renderable_texture(&display);
    let stream_0 = PrimitivesGeneratedQuery::new(&display).unwrap();
    let also_stream_0 = PrimitivesGeneratedQuery::with_stream(&display, 0).unwrap();
    let stream_1 = PrimitivesGeneratedQuery::with_stream(&display, 1).unwrap();

    let result = texture.as_surface().draw(
        &vb,
        &ib,
        &program,
        &safegl::uniforms::EmptyUniforms,
        &safegl::DrawParameters {
            primitives_generated_queries: &[&stream_0, &also_stream_0],
            ..Default::default()
        },
    );
    assert!(matches!(result, Err(safegl::DrawError::WrongQueryOperation)));

    // without a geometry shader, everything goes to stream 0
    texture
        .as_surface()
        .draw(
            &vb,
            &ib,
            &program,
            &safegl::uniforms::EmptyUniforms,
            &safegl::DrawParameters {
                primitives_generated_queries: &[&stream_0, &stream_1],
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(stream_0.get().unwrap(), 2);
    assert_eq!(stream_1.get().unwrap(), 0);

    display.assert_no_error(None);
}
