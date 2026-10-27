#[macro_use]
extern crate safegl;

use safegl::Surface;

mod support;

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

#[test]
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

    assert!(query.get().unwrap());

    let query2 = safegl::draw_parameters::AnySamplesPassedQuery::new(&display, false).unwrap();
    assert!(!query2.get().unwrap());

    display.assert_no_error(None);
}

#[test]
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

    assert!(query.get().unwrap());

    let query2 = safegl::draw_parameters::AnySamplesPassedQuery::new(&display, true).unwrap();
    assert!(!query2.get().unwrap());

    display.assert_no_error(None);
}

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
            primitives_generated_query: Some(&query),
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

#[cfg(feature = "gl46")]
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

#[cfg(feature = "gl46")]
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
