//! Filling buffers with a value (`glClearBufferSubData`).
#![cfg(safegl_gl43)]

#[macro_use]
extern crate safegl;

use safegl::buffer::{Buffer, BufferMode, BufferType, ClearBufferError};

mod support;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

type Display = support::Backend;

/// Returns false, after checking the error, if the backend can't clear buffers.
fn cleared(display: &Display, result: Result<(), ClearBufferError>) -> bool {
    match result {
        Ok(()) => true,
        Err(ClearBufferError::NotSupported) => {
            // desktop OpenGL 4.3 has it, and lower drivers through the extension
            assert!(
                support::is_gles(display)
                    || !(support::gl_at_least(display, 4, 3)
                        || support::extensions(display).gl_arb_clear_buffer_object)
            );
            false
        }
    }
}

fn buffer<T: Copy + safegl::buffer::AnyBitPattern>(
    display: &Display,
    data: &[T],
    mode: BufferMode,
) -> Buffer<[T]> {
    Buffer::new(display, data, BufferType::ArrayBuffer, mode).unwrap()
}

#[test]
fn clearing_a_whole_buffer() {
    let display = support::build_display();
    for mode in [
        BufferMode::Default,
        BufferMode::Dynamic,
        BufferMode::Immutable,
        BufferMode::Persistent,
    ] {
        let buffer = buffer(&display, &[1u32, 2, 3, 4, 5], mode);
        if !cleared(&display, buffer.clear(0xdead_beef)) {
            return;
        }
        assert_eq!(buffer.read().unwrap(), vec![0xdead_beef; 5], "{mode:?}");
    }

    display.assert_no_error(None);
}

#[test]
fn clearing_a_slice_keeps_the_rest() {
    let display = support::build_display();
    let data: Vec<[f32; 4]> = (0..6).map(|i| [i as f32; 4]).collect();
    let buffer = buffer(&display, &data, BufferMode::Default);

    let value = [0.5, -1.0, 2.0, 0.25];
    if !cleared(&display, buffer.slice(2..4).unwrap().clear(value)) {
        return;
    }
    let mut expected = data.clone();
    expected[2] = value;
    expected[3] = value;
    assert_eq!(buffer.read().unwrap(), expected);

    display.assert_no_error(None);
}

#[test]
fn clearing_a_mutable_slice() {
    let display = support::build_display();
    let mut buffer = buffer(&display, &[0u16; 8], BufferMode::Persistent);

    if !cleared(&display, buffer.slice_mut(..3).unwrap().clear(7)) {
        return;
    }
    // the persistent mapping waits for the clear
    let mapping = buffer.map_read();
    assert_eq!(&*mapping, &[7, 7, 7, 0, 0, 0, 0, 0]);

    display.assert_no_error(None);
}

#[test]
fn values_are_written_bit_for_bit() {
    let display = support::build_display();

    let bytes = buffer(&display, &[0i8; 6], BufferMode::Default);
    if !cleared(&display, bytes.clear(-3)) {
        return;
    }
    assert_eq!(bytes.read().unwrap(), vec![-3; 6]);

    let pairs = buffer(&display, &[[0u8; 2]; 3], BufferMode::Default);
    pairs.clear([0xab, 0xcd]).unwrap();
    assert_eq!(pairs.read().unwrap(), vec![[0xab, 0xcd]; 3]);

    let triples = buffer(&display, &[[0i32; 3]; 4], BufferMode::Default);
    triples.slice(1..3).unwrap().clear([-1, i32::MIN, 7]).unwrap();
    assert_eq!(
        triples.read().unwrap(),
        vec![[0; 3], [-1, i32::MIN, 7], [-1, i32::MIN, 7], [0; 3]]
    );

    // a float clears to its bits, a NaN included
    let floats = buffer(&display, &[0.0f32; 3], BufferMode::Default);
    let nan = f32::from_bits(0x7fc0_1234);
    floats.clear(nan).unwrap();
    for value in floats.read().unwrap() {
        assert_eq!(value.to_bits(), 0x7fc0_1234);
    }

    display.assert_no_error(None);
}

#[test]
fn clearing_a_single_value() {
    let display = support::build_display();
    let buffer = Buffer::new(
        &display,
        &[1i16, 2, 3, 4],
        BufferType::UniformBuffer,
        BufferMode::Default,
    )
    .unwrap();
    if !cleared(&display, buffer.clear([-5, 6, -7, 8])) {
        return;
    }
    assert_eq!(buffer.read().unwrap(), [-5, 6, -7, 8]);

    display.assert_no_error(None);
}
