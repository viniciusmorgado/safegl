#[macro_use]
extern crate safegl;

use safegl::buffer::{Buffer, BufferMode, BufferType, Content};

mod support;

struct Values {
    count: u32,
    values: [u32],
}

implement_buffer_content!(Values);

struct Generic<T> {
    values: [T],
}

implement_buffer_content!(Generic<T>);

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
