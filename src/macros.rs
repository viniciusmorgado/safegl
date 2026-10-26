//! Defines useful macros for SafeGL usage.

/// Calls the `assert_no_error` method on a `safegl::Display` instance
/// with file and line number information.
///
/// Aside from the first argument which must be the display,
/// the arguments of this macro match the `println!` macro.
///
/// ## Example
/// ```ignore rust
/// assert_no_gl_error!(my_display);
/// assert_no_gl_error!(my_display, "custom message");
/// assert_no_gl_error!(my_display, "custom format {}", 5);
/// ```
#[macro_export]
macro_rules! assert_no_gl_error {
    ($display: expr) => {
        {
            let message = format!("{}:{}", file!(), line!());
            $display.assert_no_error(Some(&message[..]));
        }
    };
    ($display: expr, $msg: expr) => {
        {
            let message = format!("{}:{}  {}", file!(), line!(), $msg);
            $display.assert_no_error(Some(&message[..]));
        }
    };
    ($display: expr, $fmt: expr, $($arg:tt)+) => {
        {
            let message = format!(concat!("{}:{} ", $fmt), file!(), line!(), $($arg)+);
            $display.assert_no_error(Some(&message[..]));
        }
    }
}

/// Returns an implementation-defined type which implements the `Uniform` trait.
///
/// ## Example
///
/// ```rust
/// # use safegl::uniform;
/// # fn main() {
/// let uniforms = uniform! {
///     color: [1.0, 1.0, 0.0, 1.0],
///     some_value: 12i32
/// };
/// # }
/// ```
///
/// Names that aren't identifiers, like the members of structs and the elements of arrays, are
/// given as strings:
///
/// ```rust
/// # use safegl::uniform;
/// # fn main() {
/// // uniform struct Light { vec3 position; vec3 color; } lights[2];
/// let uniforms = uniform! {
///     "lights[0].position": [0.0f32, 2.0, 0.0],
///     "lights[0].color": [1.0f32, 1.0, 1.0],
///     "lights[1].position": [4.0f32, 2.0, 0.0],
///     "lights[1].color": [1.0f32, 0.0, 0.0],
///     ambient: 0.1f32,
/// };
/// # }
/// ```
#[macro_export]
macro_rules! uniform {
    () => {
        $crate::uniforms::EmptyUniforms
    };

    (@name $field:ident) => {
        stringify!($field)
    };

    (@name $field:literal) => {
        $field
    };

    (@add $uniforms:expr;) => {
        $uniforms
    };

    (@add $uniforms:expr; $field:tt: $value:expr $(, $($rest:tt)*)?) => {
        $crate::uniform!(@add $uniforms.add($crate::uniform!(@name $field), $value); $($($rest)*)?)
    };

    ($field:tt: $value:expr $(, $($rest:tt)*)?) => {
        $crate::uniform!(@add
            $crate::uniforms::UniformsStorage::new($crate::uniform!(@name $field), $value);
            $($($rest)*)?
        )
    };
}

/// Returns a Dynamic Uniforms Container to which values can be added later.
///
/// ## Example
///
/// ```rust
/// # #[macro_use] extern crate safegl;
/// # use safegl::uniform;
/// # fn main(){
///     let mut uniforms = dynamic_uniform!{
///         color: &[1.0, 1.0, 0.0, 1.0],
///         some_value: &12i32,
///     };
///
///     uniforms.add("another_value", &1.5f32);
/// # }
/// ```
///
///
#[macro_export]
macro_rules! dynamic_uniform{
    () => {
        $crate::uniforms::DynamicUniforms::new()
    };

    ($($field:ident: $value:expr), *,) => {
        {
            let mut tmp = $crate::uniforms::DynamicUniforms::new();
            $(
                tmp.add(stringify!($field), $value);
            )*
            tmp
        }
    };
}

/// Implements the `safegl::vertex::Vertex` trait for the given type.
///
/// The parameters must be the name of the struct and the names of its fields.
///
/// ## Safety
///
/// You must not use this macro on any struct with fields that cannot be zeroed.
///
/// ## Example
///
/// ```
/// # use safegl::implement_vertex;
/// # fn main() {
/// #[derive(Copy, Clone)]
/// struct Vertex {
///     position: [f32; 3],
///     tex_coords: [f32; 2],
/// }
///
/// implement_vertex!(Vertex, position, tex_coords);
/// # }
/// ```
///
/// ## Naming convention
///
/// If not using the location option, when it comes to using to using your vertex array in a shader you must make sure that all your attribute variables *match* the field names in the struct you are calling calling this macro for.
///
/// So, if you have a `vertex_position` attribute/input in your shader, a field named `vertex_position` must be present in the struct. Otherwise the drawing functions will panic.
///
/// ## Normalize option
///
/// You can specify a normalize option for attributes.
/// ```
/// # #[derive(Clone, Copy)]
/// # struct Vertex {
/// #     position: [f32; 2],
/// #     tex_coords: [f32; 2],
/// # }
/// # use safegl::implement_vertex;
/// # fn main() {
/// implement_vertex!(Vertex, position normalize(false), tex_coords normalize(false));
/// # }
/// ```
/// ## Location option
///
/// You can specify a location option for attributes.
/// ```
/// # #[derive(Clone, Copy)]
/// # struct Vertex {
/// #     position: [f32; 2],
/// #     tex_coords: [f32; 2],
/// # }
/// # use safegl::implement_vertex;
/// # fn main() {
/// implement_vertex!(Vertex, position location(0), tex_coords location(1));
/// # }
/// ```
#[macro_export]
macro_rules! implement_vertex {
    ($struct_name:ty, $($field_name:ident),+) => (
        impl $struct_name {
            const BINDINGS: $crate::vertex::VertexFormat = &[
                $(
                    (
                        std::borrow::Cow::Borrowed(stringify!($field_name)),
                        ::core::mem::offset_of!($struct_name, $field_name),
                        -1,
                        {
                            const fn attr_type_of_val<T: $crate::vertex::Attribute>(_: Option<&T>)
                                -> $crate::vertex::AttributeType
                            {
                                <T as $crate::vertex::Attribute>::TYPE
                            }
                            let field_option = match None::<&$struct_name> {
                                Some(v) => Some(&v.$field_name),
                                None => None
                            };
                            attr_type_of_val(field_option)
                        },
                        false
                    )
                ),+
            ];
        }

        unsafe impl $crate::vertex::Vertex for $struct_name {
            #[inline]
            fn build_bindings() -> $crate::vertex::VertexFormat {
                Self::BINDINGS
            }
        }
    );

    ($struct_name:ty, $($field_name:ident normalize($should_normalize:expr)),+) => {
        impl $struct_name {
            const BINDINGS: $crate::vertex::VertexFormat = &[
                $(
                    (
                        std::borrow::Cow::Borrowed(stringify!($field_name)),
                        ::core::mem::offset_of!($struct_name, $field_name),
                        -1,
                        {
                            const fn attr_type_of_val<T: $crate::vertex::Attribute>(_: Option<&T>)
                                -> $crate::vertex::AttributeType
                            {
                                <T as $crate::vertex::Attribute>::TYPE
                            }
                            let field_option = match None::<&$struct_name> {
                                Some(v) => Some(&v.$field_name),
                                None => None
                            };
                            attr_type_of_val(field_option)
                        },
                        {
                            $should_normalize
                        }
                    )
                ),+
            ];
        }
        unsafe impl $crate::vertex::Vertex for $struct_name {
            #[inline]
            fn build_bindings() -> $crate::vertex::VertexFormat {
                Self::BINDINGS
            }
        }
    };

    ($struct_name:ty, $($field_name:ident location($location:expr)),+) => {
        impl $struct_name {
            const BINDINGS: $crate::vertex::VertexFormat = &[
                $(
                    (
                        std::borrow::Cow::Borrowed(stringify!($field_name)),
                        ::core::mem::offset_of!($struct_name, $field_name),
                        {
                            $location
                        },
                        {
                            const fn attr_type_of_val<T: $crate::vertex::Attribute>(_: Option<&T>)
                                -> $crate::vertex::AttributeType
                            {
                                <T as $crate::vertex::Attribute>::TYPE
                            }
                            let field_option = match None::<&$struct_name> {
                                Some(v) => Some(&v.$field_name),
                                None => None
                            };
                            attr_type_of_val(field_option)
                        },
                        false
                    )
                ),+
            ];
        }

        unsafe impl $crate::vertex::Vertex for $struct_name {
            #[inline]
            fn build_bindings() -> $crate::vertex::VertexFormat {
                Self::BINDINGS
            }
        }
    };

    ($struct_name:ty, $($field_name:ident),+,) => (
        $crate::implement_vertex!($struct_name, $($field_name),+);
    );
}

/// Implements the `safegl::buffer::AnyBitPattern` trait for the given struct.
///
/// The parameters must be the name of the struct and the names of **all** its fields, or the
/// indices of all its fields for a tuple struct. The macro fails to compile if a field is missing
/// or if the type of a field doesn't implement `AnyBitPattern`, so the struct is guaranteed to be
/// valid for every bit pattern.
///
/// Buffers can only be created without data, invalidated, read or mapped for reading when their
/// content implements `AnyBitPattern`.
///
/// ## Example
///
/// ```
/// # use safegl::implement_any_bit_pattern;
/// #[derive(Copy, Clone)]
/// struct Particle {
///     position: [f32; 3],
///     age: u32,
/// }
/// implement_any_bit_pattern!(Particle, position, age);
///
/// #[derive(Copy, Clone)]
/// struct Color(f32, f32, f32);
/// implement_any_bit_pattern!(Color, 0, 1, 2);
///
/// // unsized structs work too
/// struct Particles {
///     count: u32,
///     particles: [Particle],
/// }
/// implement_any_bit_pattern!(Particles, count, particles);
/// ```
///
/// A struct with a field that can't hold every bit pattern is rejected:
///
/// ```compile_fail
/// # use safegl::implement_any_bit_pattern;
/// #[derive(Copy, Clone)]
/// struct Flagged {
///     value: f32,
///     enabled: bool,
/// }
/// implement_any_bit_pattern!(Flagged, value, enabled);
/// ```
///
/// So is a struct whose fields aren't all listed:
///
/// ```compile_fail
/// # use safegl::implement_any_bit_pattern;
/// #[derive(Copy, Clone)]
/// struct Flagged {
///     value: f32,
///     enabled: bool,
/// }
/// implement_any_bit_pattern!(Flagged, value);
/// ```
#[macro_export]
macro_rules! implement_any_bit_pattern {
    (__underscore $index:tt) => { _ };

    (__check $struct_name:ident [$($gs:ident),*] [$($pattern:tt)*] $($field:tt),+) => {
        const _: () = {
            #[allow(dead_code, clippy::extra_unused_type_parameters)]
            fn check<$($gs: $crate::buffer::AnyBitPattern),*>(value: &$struct_name<$($gs),*>) {
                fn is_any_bit_pattern<T: ?Sized + $crate::buffer::AnyBitPattern>(_: &T) {}
                // a pattern without `..` doesn't compile if a field is missing
                let $($pattern)* = value;
                $(is_any_bit_pattern(&value.$field);)+
            }
        };

        unsafe impl<$($gs: $crate::buffer::AnyBitPattern),*> $crate::buffer::AnyBitPattern
            for $struct_name<$($gs),*>
        {
        }
    };

    ($struct_name:ident <$($gs:ident),+>, $($field_name:ident),+ $(,)?) => {
        $crate::implement_any_bit_pattern!(__check $struct_name [$($gs),+]
            [$struct_name { $($field_name: _),+ }] $($field_name),+);
    };

    ($struct_name:ident <$($gs:ident),+>, $($index:tt),+ $(,)?) => {
        $crate::implement_any_bit_pattern!(__check $struct_name [$($gs),+]
            [$struct_name($($crate::implement_any_bit_pattern!(__underscore $index)),+)]
            $($index),+);
    };

    ($struct_name:ident, $($field_name:ident),+ $(,)?) => {
        $crate::implement_any_bit_pattern!(__check $struct_name []
            [$struct_name { $($field_name: _),+ }] $($field_name),+);
    };

    ($struct_name:ident, $($index:tt),+ $(,)?) => {
        $crate::implement_any_bit_pattern!(__check $struct_name []
            [$struct_name($($crate::implement_any_bit_pattern!(__underscore $index)),+)]
            $($index),+);
    };
}

/// Implements the `safegl::buffer::Content` trait for the given type.
///
/// Contrary to the other similar macros, this one doesn't require you pass the list of parameters.
///
/// **Only use this macro on structs whose last field is a slice.** The code it generates uses
/// `std::mem::size_of_val_raw` and `std::alloc::Layout::for_value_raw`, so crates that call it
/// need Rust 1.99 or newer.
///
/// ## Example
///
/// ```
/// # use safegl::implement_buffer_content;
/// # fn main() {
/// struct Data {
///     data: [u32]
/// }
///
/// implement_buffer_content!(Data);
/// # }
/// ```
///
#[macro_export]
macro_rules! implement_buffer_content {
    (__as_item $i:item) => {$i};

    // A pointer to the struct whose slice has `len` elements. It has no address, so it can only be
    // given to functions that read its length.
    (__with_len $struct_name:ident [$($gs:tt)*] $len:expr) => {
        ::std::ptr::slice_from_raw_parts_mut(::std::ptr::null_mut::<()>(), $len)
            as *mut $struct_name<$($gs)*>
    };

    // The number of elements of a struct of `size` bytes, if there is such a struct.
    (__len $struct_name:ident [$($gs:tt)*] $size:expr) => {{
        let size: usize = $size;
        let size_of = |len: usize| unsafe {
            ::std::mem::size_of_val_raw(
                $crate::implement_buffer_content!(__with_len $struct_name [$($gs)*] len),
            )
        };
        let min_size = size_of(0);
        let step = size_of(1) - min_size;

        if size < min_size || step == 0 || (size - min_size) % step != 0 {
            None
        } else {
            // the padding at the end can make some sizes impossible
            let len = (size - min_size) / step;
            if size_of(len) == size { Some(len) } else { None }
        }
    }};

    (__impl $struct_name:ident [$($gs:tt)*]) => {
        $crate::implement_buffer_content! { __as_item
            unsafe impl<$($gs)*> $crate::buffer::Content for $struct_name<$($gs)*> {
                type Owned = Box<$struct_name<$($gs)*>>;

                #[inline]
                unsafe fn read<F, E>(size: usize, f: F) -> ::std::result::Result<Box<$struct_name<$($gs)*>>, E>
                              // the bound is higher-ranked so that it isn't rejected as trivially false
                              // for structs that don't implement the trait
                              where for<'__any> $struct_name<$($gs)*>: $crate::buffer::AnyBitPattern,
                                    F: FnOnce(&mut $struct_name<$($gs)*>) -> ::std::result::Result<(), E>
                {
                    assert!(<$struct_name<$($gs)*> as $crate::buffer::Content>::is_size_suitable(size));
                    let len = $crate::implement_buffer_content!(__len $struct_name [$($gs)*] size)
                        .unwrap();

                    // the value is allocated with its own layout, so that `Box` can free it
                    let layout = unsafe {
                        ::std::alloc::Layout::for_value_raw(
                            $crate::implement_buffer_content!(__with_len $struct_name [$($gs)*] len),
                        )
                    };
                    let data = unsafe { ::std::alloc::alloc_zeroed(layout) };
                    if data.is_null() {
                        ::std::alloc::handle_alloc_error(layout);
                    }

                    let ptr = ::std::ptr::slice_from_raw_parts_mut(data as *mut (), len)
                        as *mut $struct_name<$($gs)*>;
                    let mut storage = unsafe { Box::from_raw(ptr) };

                    f(&mut storage)?;
                    Ok(storage)
                }

                #[inline]
                fn get_elements_size() -> usize {
                    unsafe {
                        ::std::mem::size_of_val_raw(
                            $crate::implement_buffer_content!(__with_len $struct_name [$($gs)*] 0),
                        )
                    }
                }

                #[inline]
                fn to_void_ptr(&self) -> *const () {
                    self as *const Self as *const ()
                }

                #[inline]
                fn ref_from_ptr(ptr: *mut (), size: usize) -> Option<*mut $struct_name<$($gs)*>> {
                    let len = $crate::implement_buffer_content!(__len $struct_name [$($gs)*] size)?;
                    Some(::std::ptr::slice_from_raw_parts_mut(ptr, len) as *mut $struct_name<$($gs)*>)
                }

                #[inline]
                fn is_size_suitable(size: usize) -> bool {
                    let min_size = <$struct_name<$($gs)*> as $crate::buffer::Content>::get_elements_size();
                    size > min_size
                        && $crate::implement_buffer_content!(__len $struct_name [$($gs)*] size).is_some()
                }
            }
        }
    };

    ($struct_name:ident,) => (
        $crate::implement_buffer_content!($struct_name);
    );

    ($struct_name:ident) => (
        $crate::implement_buffer_content!(__impl $struct_name []);
    );

    ($struct_name:ident <$t1:tt>) => (
        $crate::implement_buffer_content!(__impl $struct_name [$t1]);
    );
}

/// Implements the `safegl::uniforms::UniformBlock` trait for the given type.
///
/// The parameters must be the name of the struct and the names of its fields.
///
/// ## Example
///
/// ```
/// # use safegl::implement_uniform_block;
/// # fn main() {
/// #[derive(Copy, Clone)]
/// struct Vertex {
///     value1: [f32; 3],
///     value2: [f32; 2],
/// }
///
/// implement_uniform_block!(Vertex, value1, value2);
/// # }
/// ```
///
#[macro_export]
macro_rules! implement_uniform_block {
    (__as_item $i:item) => {$i};

    (__impl $struct_name:ident [$($gs:tt)*], $($field_name:ident),+) => (
        $crate::implement_uniform_block! { __as_item
            unsafe impl<$($gs)*> $crate::uniforms::UniformBlock for $struct_name<$($gs)*> {
                fn matches(layout: &$crate::program::BlockLayout, base_offset: usize)
                           -> ::std::result::Result<(), $crate::uniforms::LayoutMismatchError>
                {
                    use std::mem;
                    use $crate::program::BlockLayout;
                    use $crate::uniforms::LayoutMismatchError;

                    if let &BlockLayout::Struct { ref members } = layout {
                        // checking that each member exists in the input struct
                        for &(ref name, _) in members {
                            if $(name != stringify!($field_name) &&)+ true {
                                return Err(LayoutMismatchError::MissingField {
                                    name: name.clone(),
                                });
                            }
                        }

                        fn matches_from_ty<T: $crate::uniforms::UniformBlock + ?Sized>(_: Option<&T>,
                            layout: &$crate::program::BlockLayout, base_offset: usize)
                            -> ::std::result::Result<(), $crate::uniforms::LayoutMismatchError>
                        {
                            <T as $crate::uniforms::UniformBlock>::matches(layout, base_offset)
                        }

                        // checking that each field of the input struct is correct in the reflection
                        $(
                            let reflected_ty = members.iter().find(|&&(ref name, _)| {
                                                                        name == stringify!($field_name)
                                                                   });
                            let reflected_ty = match reflected_ty {
                                Some(t) => &t.1,
                                None => return Err(LayoutMismatchError::MissingField {
                                    name: stringify!($field_name).to_owned(),
                                })
                            };

                            let input_offset = mem::offset_of!($struct_name, $field_name);
                            let dummy_field = None::<&$struct_name>.map(|v| &v.$field_name);

                            match matches_from_ty(dummy_field, reflected_ty, input_offset) {
                                Ok(_) => (),
                                Err(e) => return Err(LayoutMismatchError::MemberMismatch {
                                    member: stringify!($field_name).to_owned(),
                                    err: Box::new(e),
                                })
                            };
                        )+

                        Ok(())

                    } else {
                        Err(LayoutMismatchError::LayoutMismatch {
                            expected: layout.clone(),
                            obtained: <Self as $crate::uniforms::UniformBlock>::build_layout(base_offset),
                        })
                    }
                }

                fn build_layout(base_offset: usize) -> $crate::program::BlockLayout {
                    use $crate::program::BlockLayout;

                    fn layout_from_ty<T: $crate::uniforms::UniformBlock + ?Sized>(_: Option<&T>, base_offset: usize)
                                                                         -> BlockLayout
                    {
                        <T as $crate::uniforms::UniformBlock>::build_layout(base_offset)
                    }

                    BlockLayout::Struct {
                        members: vec![
                            $(
                                (
                                    stringify!($field_name).to_owned(),
                                    {
                                        let offset = $crate::__safegl_offset_of!($struct_name, $field_name);
                                        let field_option = None::<&$struct_name>.map(|v| &v.$field_name);
                                        layout_from_ty(field_option, offset + base_offset)
                                    }
                                ),
                            )+
                        ],
                    }
                }
            }
        }
    );

    ($struct_name:ident, $($field_name:ident),+,) => (
        $crate::implement_uniform_block!($struct_name, $($field_name),+);
    );

    ($struct_name:ident, $($field_name:ident),+) => (
        $crate::implement_uniform_block!(__impl $struct_name [], $($field_name),+);
    );

    ($struct_name:ident<$l:tt>, $($field_name:ident),+) => (
        $crate::implement_uniform_block!(__impl $struct_name [$l], $($field_name),+);
    );
}

/// Builds a program depending on the GLSL version supported by the backend.
///
/// This is implemented with successive calls to `is_glsl_version_supported()`.
///
/// Returns a `safegl::program::ProgramChooserCreationError`.
///
/// ## Example
///
/// ```no_run
/// use safegl::program;
/// # use glutin::surface::{ResizeableSurface, SurfaceTypeTrait};
/// # fn example<T>(display: safegl::Display<T>) where T: SurfaceTypeTrait + ResizeableSurface {
/// let program = program!(&display,
///     300 => {
///         vertex: r#"
///             #version 300
///
///             void main() {
///                 gl_Position = vec4(0.0, 0.0, 0.0, 1.0);
///             }
///         "#,
///         fragment: r#"
///             #version 300
///
///             out vec4 color;
///             void main() {
///                 color = vec4(1.0, 1.0, 0.0, 1.0);
///             }
///         "#,
///     },
///     110 => {
///         vertex: r#"
///             #version 110
///
///             void main() {
///                 gl_Position = vec4(0.0, 0.0, 0.0, 1.0);
///             }
///         "#,
///         fragment: r#"
///             #version 110
///
///             void main() {
///                 gl_FragColor = vec4(1.0, 1.0, 0.0, 1.0);
///             }
///         "#,
///     },
///     300 es => {
///         vertex: r#"
///             #version 110
///
///             void main() {
///                 gl_Position = vec4(0.0, 0.0, 0.0, 1.0);
///             }
///         "#,
///         fragment: r#"
///             #version 110
///
///             void main() {
///                 gl_FragColor = vec4(1.0, 1.0, 0.0, 1.0);
///             }
///         "#,
///     },
/// );
/// # }
/// ```
///
#[macro_export]
macro_rules! program {
    ($facade:expr,) => (
        Err($crate::program::ProgramChooserCreationError::NoVersion)
    );

    ($facade:expr,,$($rest:tt)*) => (
        $crate::program!($facade,$($rest)*)
    );

    ($facade:expr, $num:tt => $($rest:tt)*) => (
        {
            let context = $crate::backend::Facade::get_context($facade);
            let version = $crate::program!(_parse_num_gl $num);
            $crate::program!(_inner, context, version, $($rest)*)
        }
    );

    ($facade:expr, $num:tt es => $($rest:tt)*) => (
        {
            let context = $crate::backend::Facade::get_context($facade);
            let version = $crate::program!(_parse_num_gles $num);
            $crate::program!(_inner, context, version, $($rest)*)
        }
    );

    (_inner, $context:ident, $vers:ident, {$($ty:ident:$src:expr),+}$($rest:tt)*) => (
        if $context.is_glsl_version_supported(&$vers) {
            let __vertex_shader: &str = "";
            let __tessellation_control_shader: Option<&str> = None;
            let __tessellation_evaluation_shader: Option<&str> = None;
            let __geometry_shader: Option<&str> = None;
            let __fragment_shader: &str = "";
            let __outputs_srgb: bool = true;
            let __uses_point_size: bool = false;

            $(
                $crate::program!(_program_ty $ty, $src, __vertex_shader, __tessellation_control_shader,
                         __tessellation_evaluation_shader, __geometry_shader, __fragment_shader,
                         __outputs_srgb, __uses_point_size);
            )+

            let input = $crate::program::ProgramCreationInput::SourceCode {
                vertex_shader: __vertex_shader,
                tessellation_control_shader: __tessellation_control_shader,
                tessellation_evaluation_shader: __tessellation_evaluation_shader,
                geometry_shader: __geometry_shader,
                fragment_shader: __fragment_shader,
                transform_feedback_varyings: None,
                outputs_srgb: __outputs_srgb,
                uses_point_size: __uses_point_size,
            };

            $crate::program::Program::new($context, input)
                           .map_err(|err| $crate::program::ProgramChooserCreationError::from(err))

        } else {
            $crate::program!($context, $($rest)*)
        }
    );

    (_inner, $context:ident, $vers:ident, {$($ty:ident:$src:expr),+,}$($rest:tt)*) => (
        $crate::program!(_inner, $context, $vers, {$($ty:$src),+} $($rest)*)
    );

    (_program_ty vertex, $src:expr, $vs:ident, $tcs:ident, $tes:ident, $gs:ident, $fs:ident, $srgb:ident, $ps:ident) => (
        let $vs = $src;
    );

    (_program_ty tessellation_control, $src:expr, $vs:ident, $tcs:ident, $tes:ident, $gs:ident, $fs:ident, $srgb:ident, $ps:ident) => (
        let $tcs = Some($src);
    );

    (_program_ty tessellation_evaluation, $src:expr, $vs:ident, $tcs:ident, $tes:ident, $gs:ident, $fs:ident, $srgb:ident, $ps:ident) => (
        let $tes = Some($src);
    );

    (_program_ty geometry, $src:expr, $vs:ident, $tcs:ident, $tes:ident, $gs:ident, $fs:ident, $srgb:ident, $ps:ident) => (
        let $gs = Some($src);
    );

    (_program_ty fragment, $src:expr, $vs:ident, $tcs:ident, $tes:ident, $gs:ident, $fs:ident, $srgb:ident, $ps:ident) => (
        let $fs = $src;
    );

    (_program_ty point_size, $src:expr, $vs:ident, $tcs:ident, $tes:ident, $gs:ident, $fs:ident, $srgb:ident, $ps:ident) => (
        let $ps = $src;
    );

    (_program_ty outputs_srgb, $src:expr, $vs:ident, $tcs:ident, $tes:ident, $gs:ident, $fs:ident, $srgb:ident, $ps:ident) => (
        let $srgb = $src;
    );

    (_parse_num_gl $num:expr) => (
        if $num == 100 {
            $crate::Version($crate::Api::GlEs, 1, 0)
        } else {
            let num: u32 = $num;
            $crate::Version($crate::Api::Gl, (num / 100) as u8, ((num % 100) / 10) as u8)
        }
    );

    (_parse_num_gles $num:expr) => ({
        let num: u32 = $num;
        $crate::Version($crate::Api::GlEs, (num / 100) as u8, ((num % 100) / 10) as u8)
    });
}

#[cfg(test)]
mod tests {
    #[test]
    fn trailing_comma_impl_uniforms() {
        let u = uniform! { a: 5, b: 6, };
    }

    #[test]
    fn trailing_comma_impl_vertex() {
        #[derive(Copy, Clone)]
        struct Foo {
            pos: [f32; 2],
        }

        implement_vertex!(Foo, pos,);
    }

    #[test]
    fn assert_no_error_macro() {
        struct Dummy;
        impl Dummy {
            fn assert_no_error(&self, _: Option<&str>) {}
        }

        assert_no_gl_error!(Dummy);

        assert_no_gl_error!(Dummy, "hi");

        assert_no_gl_error!(Dummy, "{} {}", 1, 2);
    }
}
