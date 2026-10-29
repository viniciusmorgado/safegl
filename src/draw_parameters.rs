//! Describes miscellaneous parameters to be used when drawing.
//!
//! # Example
//!
//! ```rust
//! let params = safegl::DrawParameters {
//!     depth: safegl::Depth {
//!         test: safegl::draw_parameters::DepthTest::IfLess,
//!         write: true,
//!         .. Default::default()
//!     },
//!     scissor: Some(safegl::Rect { bottom: 0, left: 100, width: 100, height: 200 }),
//!     .. Default::default()
//! };
//! ```
//!
//! # Queries
//!
//! Query objects allow you to obtain information about the rendering process. For example, an
//! `AnySamplesPassedQuery` allows you to know whether any sample has been drawn, and a
//! `SamplesPassedQuery` (desktop OpenGL levels) how many.
//!
//! ```no_run
//! # use glutin::surface::{ResizeableSurface, SurfaceTypeTrait};
//! # fn example<T>(display: safegl::Display<T>) where T: SurfaceTypeTrait + ResizeableSurface {
//! let query = safegl::draw_parameters::AnySamplesPassedQuery::new(&display, false).unwrap();
//! let params = safegl::DrawParameters {
//!     samples_passed_query: Some((&query).into()),
//!     .. Default::default()
//! };
//! # }
//! ```
//!
//! After drawing with these parameters, you can retrieve the value inside the query:
//!
//! ```no_run
//! # fn example(query: safegl::draw_parameters::AnySamplesPassedQuery) {
//! let value = query.get();
//! # }
//! ```
//!
//! This operation will consume the query and block until the GPU has finished drawing. Instead,
//! at the desktop OpenGL levels, you can also use the query as a condition for drawing:
//!
//! ```no_run
//! # fn example(query: safegl::draw_parameters::AnySamplesPassedQuery) {
//! # #[cfg(safegl_desktop)] {
//! let params = safegl::DrawParameters {
//!     condition: Some(safegl::draw_parameters::ConditionalRendering {
//!         query: (&query).into(),
//!         wait: true,
//!         per_region: true,
//!         # #[cfg(safegl_gl45)]
//!         # inverted: false,
//!     }),
//!     .. Default::default()
//! };
//! # }
//! # }
//! ```
//!
//! If you use conditional rendering, SafeGL will submit the draw command but the GPU will execute
//! it only if the query contains a value different from 0.
//!
//! ## WrongQueryOperation errors
//!
//! OpenGL puts some restrictions about the usage of queries. If you draw one or several times
//! with a query, then draw *without* that query, then the query cannot be used again. Trying
//! to draw with it results in a `WrongQueryOperation` error returned by the `draw` function.
//!
//! For the same reasons, as soon as you call `is_ready` on a query it will stop being usable.

use crate::context;
use crate::context::Context;
use crate::gl;
use crate::version::is_webgl;
use crate::version::Api;
use crate::version::Version;
use glow::HasContext;

use crate::index::PrimitiveType;

use crate::vertex::TransformFeedback;
use crate::DrawError;
use crate::GlObject;
use crate::QueryExt;
use crate::Rect;
#[cfg(any(safegl_desktop, safegl_webgl))]
use crate::ToGlEnum;

#[cfg(safegl_gles32)]
use std::ops::Range;

pub use self::blend::{Blend, BlendingFunction, LinearBlendingFactor};
#[cfg(any(safegl_desktop, safegl_webgl))]
pub use self::depth::DepthClamp;
pub use self::depth::{Depth, DepthTest};
#[cfg(any(safegl_desktop, safegl_gles32))]
pub use self::query::PrimitivesGeneratedQuery;
pub use self::query::QueryCreationError;
pub use self::query::QueryResultError;
#[cfg(safegl_gl44)]
#[cfg_attr(docsrs, doc(cfg(safegl_gl44)))]
pub use self::query::ToBufferError;
#[cfg(safegl_gl44)]
#[cfg_attr(docsrs, doc(cfg(safegl_gl44)))]
pub use self::query::{QueryBufferMode, QueryBufferValue};
#[cfg(safegl_gl44)]
pub(crate) use self::query::write_query_to_buffer;
pub use self::query::{AnySamplesPassedQuery, TransformFeedbackPrimitivesWrittenQuery};
#[cfg(safegl_desktop)]
pub use self::query::SamplesPassedQuery;
#[cfg(any(safegl_desktop, safegl_webgl))]
pub use self::query::TimeElapsedQuery;
pub(crate) use self::query::{check_disjoint, sync_indexed_queries, RawQuery};
#[cfg(safegl_gl46)]
#[cfg_attr(docsrs, doc(cfg(safegl_gl46)))]
pub use self::query::{PipelineStatistic, PipelineStatisticsQuery, TransformFeedbackOverflowQuery};
pub use self::stencil::{Stencil, StencilOperation, StencilTest};

mod blend;
mod depth;
mod indexed;
mod query;
mod stencil;

/// Describes how triangles should be filtered before the fragment processing. Backface culling
/// is purely an optimization. If you don't know what this does, just use `CullingDisabled`.
///
/// # Backface culling
///
/// After the vertex shader stage, the GPU knows the 2D coordinates of each vertex of
/// each triangle.
///
/// For a given triangle, there are only two situations:
///
/// - The vertices are arranged in a clockwise direction on the screen.
/// - The vertices are arranged in a counterclockwise direction on the screen.
///
/// If you wish so, you can ask the GPU to discard all the primitives that belong to one
/// of these two categories.
///
/// ## Example
///
/// The vertices of this triangle are counter-clock-wise.
///
/// <svg width="556.84381" height="509.69049" version="1.1">
///   <g transform="translate(-95.156215,-320.37201)">
///     <path style="fill:none;stroke:#000000;stroke-width:4;stroke-miterlimit:4;stroke-opacity:1;stroke-dasharray:none" d="M 324.25897,418.99654 539.42145,726.08292 212.13204,741.23521 z" />
///     <text style="font-size:40px;font-style:normal;font-weight:normal;line-height:125%;letter-spacing:0px;word-spacing:0px;fill:#000000;fill-opacity:1;stroke:none;font-family:Sans" x="296.98483" y="400.81378"><tspan x="296.98483" y="400.81378">1</tspan></text>
///     <text style="font-size:40px;font-style:normal;font-weight:normal;line-height:125%;letter-spacing:0px;word-spacing:0px;fill:#000000;fill-opacity:1;stroke:none;font-family:Sans" x="175.22902" y="774.8031"><tspan x="175.22902" y="774.8031">2</tspan></text>
///     <text style="font-size:40px;font-style:normal;font-weight:normal;line-height:125%;letter-spacing:0px;word-spacing:0px;fill:#000000;fill-opacity:1;stroke:none;font-family:Sans" x="555.58386" y="748.30627"><tspan x="555.58386" y="748.30627">3</tspan></text>
///   </g>
/// </svg>
///
/// # Usage
///
/// The trick is that if you make a 180° rotation of a shape, all triangles that were
/// clockwise become counterclockwise and vice versa.
///
/// Therefore you can arrange your model so that the triangles that are facing the screen
/// are all either clockwise or counterclockwise, and all the triangle are *not* facing
/// the screen are the other one.
///
/// By doing so you can use backface culling to discard all the triangles that are not
/// facing the screen, and increase your framerate.
///
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BackfaceCullingMode {
    /// All triangles are always drawn.
    CullingDisabled,

    /// Triangles whose vertices are counterclockwise won't be drawn.
    CullCounterClockwise,

    /// Triangles whose vertices are clockwise won't be drawn.
    CullClockwise,
}

/// Defines how the device should render polygons.
///
/// The usual value is `Fill`, which fills the content of polygon with the color. However other
/// values are sometimes useful, especially for debugging purposes.
///
/// # Example
///
/// The same triangle drawn respectively with `Fill`, `Line` and `Point` (barely visible).
///
/// <svg width="890.26135" height="282.59375" version="1.1">
///  <g transform="translate(0,-769.9375)">
///     <path style="fill:#ff0000;fill-opacity:1;stroke:none" d="M 124.24877,771.03979 258.59906,1051.8622 0,1003.3749 z" />
///     <path style="fill:none;fill-opacity:1;stroke:#ff0000;stroke-opacity:1" d="M 444.46713,771.03979 578.81742,1051.8622 320.21836,1003.3749 z" />
///     <path style="fill:#ff0000;fill-opacity:1;stroke:none" d="m 814.91074,385.7662 c 0,0.0185 -0.015,0.0335 -0.0335,0.0335 -0.0185,0 -0.0335,-0.015 -0.0335,-0.0335 0,-0.0185 0.015,-0.0335 0.0335,-0.0335 0.0185,0 0.0335,0.015 0.0335,0.0335 z" transform="matrix(18.833333,0,0,18.833333,-14715.306,-6262.0056)" />
///     <path style="fill:#ff0000;fill-opacity:1;stroke:none" d="m 814.91074,385.7662 c 0,0.0185 -0.015,0.0335 -0.0335,0.0335 -0.0185,0 -0.0335,-0.015 -0.0335,-0.0335 0,-0.0185 0.015,-0.0335 0.0335,-0.0335 0.0185,0 0.0335,0.015 0.0335,0.0335 z" transform="matrix(18.833333,0,0,18.833333,-14591.26,-6493.994)" />
///     <path style="fill:#ff0000;fill-opacity:1;stroke:none" d="m 814.91074,385.7662 c 0,0.0185 -0.015,0.0335 -0.0335,0.0335 -0.0185,0 -0.0335,-0.015 -0.0335,-0.0335 0,-0.0185 0.015,-0.0335 0.0335,-0.0335 0.0185,0 0.0335,0.015 0.0335,0.0335 z" transform="matrix(18.833333,0,0,18.833333,-14457.224,-6213.6135)" />
///  </g>
/// </svg>
///
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg(any(safegl_desktop, safegl_webgl))]
#[cfg_attr(docsrs, doc(cfg(any(safegl_desktop, safegl_webgl))))]
pub enum PolygonMode {
    /// Only draw a single point at each vertex.
    ///
    /// All attributes that apply to points (`point_size`) are used when using this mode.
    Point,

    /// Only draw a line in the boundaries of each polygon.
    ///
    /// All attributes that apply to lines (`line_width`) are used when using this mode.
    Line,

    /// Fill the content of the polygon. This is the default mode.
    Fill,
}

#[cfg(any(safegl_desktop, safegl_webgl))]
impl ToGlEnum for PolygonMode {
    #[inline]
    fn to_glenum(&self) -> gl::types::GLenum {
        match *self {
            PolygonMode::Point => gl::POINT,
            PolygonMode::Line => gl::LINE,
            PolygonMode::Fill => gl::FILL,
        }
    }
}

/// The corner of points where `gl_PointCoord` is `(0, 0)`. See
/// `DrawParameters::point_coord_origin`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[cfg(safegl_desktop)]
#[cfg_attr(docsrs, doc(cfg(safegl_desktop)))]
pub enum PointCoordOrigin {
    /// The upper left corner, so `gl_PointCoord.y` grows downwards. This is the default, and the
    /// only origin of OpenGL ES.
    #[default]
    UpperLeft,
    /// The lower left corner, so `gl_PointCoord.y` grows upwards like window coordinates.
    LowerLeft,
}

/// A bitwise operation between the color that a fragment writes (`s`) and the color already in
/// the target (`d`). See `DrawParameters::logic_op`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg(safegl_desktop)]
#[cfg_attr(docsrs, doc(cfg(safegl_desktop)))]
pub enum LogicOp {
    /// `0`
    Clear,
    /// `s & d`
    And,
    /// `s & !d`
    AndReverse,
    /// `s`, what is written without a logic operation.
    Copy,
    /// `!s & d`
    AndInverted,
    /// `d`: nothing is written.
    Noop,
    /// `s ^ d`
    Xor,
    /// `s | d`
    Or,
    /// `!(s | d)`
    Nor,
    /// `!(s ^ d)`
    Equiv,
    /// `!d`
    Invert,
    /// `s | !d`
    OrReverse,
    /// `!s`
    CopyInverted,
    /// `!s | d`
    OrInverted,
    /// `!(s & d)`
    Nand,
    /// All the bits set.
    Set,
}

#[cfg(safegl_desktop)]
impl ToGlEnum for LogicOp {
    #[inline]
    fn to_glenum(&self) -> gl::types::GLenum {
        match *self {
            LogicOp::Clear => gl::CLEAR,
            LogicOp::And => gl::AND,
            LogicOp::AndReverse => gl::AND_REVERSE,
            LogicOp::Copy => gl::COPY,
            LogicOp::AndInverted => gl::AND_INVERTED,
            LogicOp::Noop => gl::NOOP,
            LogicOp::Xor => gl::XOR,
            LogicOp::Or => gl::OR,
            LogicOp::Nor => gl::NOR,
            LogicOp::Equiv => gl::EQUIV,
            LogicOp::Invert => gl::INVERT,
            LogicOp::OrReverse => gl::OR_REVERSE,
            LogicOp::CopyInverted => gl::COPY_INVERTED,
            LogicOp::OrInverted => gl::OR_INVERTED,
            LogicOp::Nand => gl::NAND,
            LogicOp::Set => gl::SET,
        }
    }
}

/// Specifies a hint for the smoothing.
///
/// Note that this is just a hint and the driver may disregard it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg(safegl_desktop)]
#[cfg_attr(docsrs, doc(cfg(safegl_desktop)))]
pub enum Smooth {
    /// The most efficient option should be chosen.
    Fastest,

    /// The most correct, or highest quality, option should be chosen.
    Nicest,

    /// No preference.
    DontCare,
}

#[cfg(safegl_desktop)]
impl ToGlEnum for Smooth {
    #[inline]
    fn to_glenum(&self) -> gl::types::GLenum {
        match *self {
            Smooth::Fastest => gl::FASTEST,
            Smooth::Nicest => gl::NICEST,
            Smooth::DontCare => gl::DONT_CARE,
        }
    }
}

/// The vertex to use for flat shading.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg(any(safegl_desktop, safegl_webgl))]
#[cfg_attr(docsrs, doc(cfg(any(safegl_desktop, safegl_webgl))))]
pub enum ProvokingVertex {
    /// Use the last vertex of each primitive.
    LastVertex,

    /// Use the first vertex of each primitive.
    ///
    /// Note that for triangle fans, this is not the first vertex but the second vertex.
    FirstVertex,
}

/// Represents the parameters to use when drawing.
///
/// Example:
///
/// ```
/// let params = safegl::DrawParameters {
///     depth: safegl::Depth {
///         test: safegl::DepthTest::IfLess,
///         write: true,
///         .. Default::default()
///     },
///     .. Default::default()
/// };
/// ```
///
#[derive(Clone, Debug)]
pub struct DrawParameters<'a> {
    /// How the fragment will interact with the depth buffer.
    pub depth: Depth,

    /// How the fragment will interact with the stencil buffer.
    pub stencil: Stencil,

    /// The effect that the GPU will use to merge the existing pixel with the pixel that is
    /// being written.
    pub blend: Blend,

    /// Allows you to disable some color components.
    ///
    /// This affects all attachments to the framebuffer. It's at the same level as the
    /// blending function.
    ///
    /// The parameters are in order: red, green, blue, alpha. `true` means that the given
    /// component will be written, `false` means that it will be ignored. The default value
    /// is `(true, true, true, true)`.
    pub color_mask: (bool, bool, bool, bool),

    /// Blending and color mask of each output of the fragment shader, overriding `blend` and
    /// `color_mask` for it. The default value is an empty slice, where every output uses
    /// `blend` and `color_mask`.
    ///
    /// Entry `i` applies to the output at location `i`, which is draw buffer `i` of the
    /// framebuffer: the location that the program gives to an output's name with a
    /// `MultiOutputFrameBuffer`, and location 0 with a `SimpleFrameBuffer`. Outputs past the
    /// end of the slice use `blend` and `color_mask`. Typical uses are a G-buffer where one
    /// attachment blends and the others don't, and weighted blended order-independent
    /// transparency, whose two outputs blend differently.
    ///
    /// OpenGL has a single blend constant: the entries whose factors use it must have the same
    /// `Blend::constant_value` as each other and as `blend`, if it uses one, or
    /// `DrawError::BlendingParameterNotSupported` is returned.
    ///
    /// More entries than `Capabilities::max_draw_buffers` return
    /// `DrawError::OutputStateIndexOutOfBounds`. Needs OpenGL 4.0, OpenGL ES 3.2, or
    /// `GL_EXT_draw_buffers_indexed` or `GL_OES_draw_buffers_indexed` on OpenGL ES 3.0 and 3.1
    /// (`OES_draw_buffers_indexed` on WebGL). Elsewhere, an entry that differs from `blend` and
    /// `color_mask` returns `DrawError::PerOutputStateNotSupported`.
    pub outputs: &'a [OutputState],

    /// A bitwise operation that replaces the color written into the color buffers
    /// (`glLogicOp`). Default value is `None`, which writes the colors as they are.
    ///
    /// It applies to the normalized and integer color buffers. Floating-point buffers, and sRGB
    /// buffers when sRGB conversion is enabled, are written as if it were `None`. The color mask
    /// still applies.
    ///
    /// OpenGL disables blending on the buffers that the operation applies to, so a logic
    /// operation with any blending in `blend` or `outputs` returns
    /// `DrawError::LogicOpWithBlending`. OpenGL ES has no logic operations: drawing into an
    /// OpenGL ES context with one returns `DrawError::LogicOpNotSupported`.
    #[cfg(safegl_desktop)]
    #[cfg_attr(docsrs, doc(cfg(safegl_desktop)))]
    pub logic_op: Option<LogicOp>,

    /// Width in pixels of the lines to draw when drawing lines.
    ///
    /// `None` means "don't care". Use this when you don't draw lines.
    ///
    /// Desktop OpenGL only draws lines of 1 pixel: wide lines are deprecated, and removed from
    /// forward-compatible contexts, so a width above 1 returns `DrawError::LineWidthNotSupported`.
    /// OpenGL ES draws wider lines up to a maximum of the implementation.
    pub line_width: Option<f32>,

    /// Diameter in pixels of the points to draw when drawing points.
    ///
    /// `None` means "don't care". Use this when you don't draw points.
    ///
    /// Ignored on OpenGL ES and WebGL, where the vertex shader must write the size of points
    /// to `gl_PointSize`.
    #[cfg(safegl_desktop)]
    #[cfg_attr(docsrs, doc(cfg(safegl_desktop)))]
    pub point_size: Option<f32>,

    /// The corner of points where `gl_PointCoord` is `(0, 0)` (`GL_POINT_SPRITE_COORD_ORIGIN`).
    /// Default value is `PointCoordOrigin::UpperLeft`.
    ///
    /// OpenGL ES always uses the upper left corner: `LowerLeft` returns
    /// `DrawError::PointCoordOriginNotSupported` when drawing into an OpenGL ES context.
    #[cfg(safegl_desktop)]
    #[cfg_attr(docsrs, doc(cfg(safegl_desktop)))]
    pub point_coord_origin: PointCoordOrigin,

    /// The size in pixels under which multisampled points fade instead of getting smaller
    /// (`GL_POINT_FADE_THRESHOLD_SIZE`). Default value is `1.0`; negative values count as 0.
    ///
    /// Ignored on OpenGL ES, which doesn't fade points.
    #[cfg(safegl_desktop)]
    #[cfg_attr(docsrs, doc(cfg(safegl_desktop)))]
    pub point_fade_threshold: f32,

    /// If the bit corresponding to 2^i is 1 in the bitmask, then GL_CLIP_DISTANCEi is enabled.
    ///
    /// The most common value for GL_MAX_CLIP_DISTANCES is 8, so 32 bits in the mask is plenty.
    ///
    /// See `https://www.khronos.org/registry/OpenGL-Refpages/gl4/html/gl_ClipDistance.xhtml`.
    ///
    /// OpenGL ES needs `GL_EXT_clip_cull_distance`, and WebGL `WEBGL_clip_cull_distance`, whose
    /// shaders write `gl_ClipDistance` with `#extension GL_ANGLE_clip_cull_distance : require`.
    /// Without them, enabling a plane returns `DrawError::ClipPlaneIndexOutOfBounds`.
    #[cfg(any(safegl_desktop, safegl_webgl))]
    #[cfg_attr(docsrs, doc(cfg(any(safegl_desktop, safegl_webgl))))]
    pub clip_planes_bitmask: u32,

    /// Whether or not the GPU should filter out some faces.
    ///
    /// After the vertex shader stage, the GPU will try to remove the faces that aren't facing
    /// the camera.
    ///
    /// See the `BackfaceCullingMode` documentation for more infos.
    pub backface_culling: BackfaceCullingMode,

    /// The winding of the triangles that face the camera (`glFrontFace`), whose fragments have
    /// `gl_FrontFacing` true. `backface_culling` and the two-sided stencil settings still name the
    /// triangles by their winding; points and lines use the stencil settings of the front faces.
    pub front_face: FrontFace,

    /// How to render polygons. The default value is `Fill`.
    ///
    /// See the documentation of `PolygonMode` for more infos.
    ///
    /// OpenGL ES only supports `Fill`, and WebGL `Line` too with `WEBGL_polygon_mode`; other
    /// values return `DrawError::PolygonModeNotSupported`.
    #[cfg(any(safegl_desktop, safegl_webgl))]
    #[cfg_attr(docsrs, doc(cfg(any(safegl_desktop, safegl_webgl))))]
    pub polygon_mode: PolygonMode,

    /// The multisampling of the draw: whether it is used, how the alpha and the coverage of the
    /// fragments choose the samples they write, and how often the fragment shader runs.
    pub multisample: MultisampleState,

    /// Whether dithering is activated. Default value is `true`.
    ///
    /// Dithering will smoothen the transition between colors in your color buffer.
    pub dithering: bool,

    /// The viewport to use when drawing.
    ///
    /// The X and Y positions of your vertices are mapped to the viewport so that `(-1, -1)`
    /// corresponds to the lower-left hand corner and `(1, 1)` corresponds to the top-right
    /// hand corner. Any pixel outside of the viewport is discarded.
    ///
    /// You can specify a viewport greater than the target if you want to stretch the image.
    ///
    /// `None` means "use the whole surface".
    pub viewport: Option<Rect>,

    /// If specified, only pixels in this rect will be displayed. Default is `None`.
    ///
    /// This is different from a viewport. The image will stretch to fill the viewport, but
    /// not the scissor box.
    pub scissor: Option<Rect>,

    /// Several viewports, each with its depth range and scissor box, among which a geometry
    /// shader chooses by writing `gl_ViewportIndex` (`glViewportArrayv`, `glDepthRangeArrayv`
    /// and `glScissorArrayv`). Default value is an empty slice, which uses `viewport`,
    /// `scissor` and `Depth::range` instead.
    ///
    /// Primitives whose `gl_ViewportIndex` is past the end of the slice use the whole surface,
    /// the `(0.0, 1.0)` depth range and no scissor box. Typical uses are rendering the faces
    /// of a cube map or the cascades of a shadow map into the parts of an atlas in one draw.
    ///
    /// With a non-empty slice, `viewport` and `scissor` must be `None` and `Depth::range` must
    /// be `(0.0, 1.0)`, or `DrawError::ViewportArrayConflict` is returned. More entries than
    /// `Limits::max_viewports` return `DrawError::ViewportIndexOutOfBounds`. Needs OpenGL 4.1
    /// or `GL_ARB_viewport_array`, otherwise returns `DrawError::ViewportArrayNotSupported`.
    #[cfg(safegl_gl41)]
    #[cfg_attr(docsrs, doc(cfg(safegl_gl41)))]
    pub viewports: &'a [ViewportState],

    /// If `false`, the pipeline will stop after the primitives generation stage. The default
    /// value is `true`.
    ///
    /// If `false`, the fragment shader of your program won't be executed.
    ///
    /// If `false`, drawing may return `TransformFeedbackNotSupported` if the backend doesn't
    /// support this feature.
    ///
    /// This parameter may seem pointless, but it can be useful when you use transform
    /// feedback or if you just use your shaders to write to a buffer.
    pub draw_primitives: bool,

    /// If set, each sample (ie. usually each pixel) written to the output adds one to the
    /// counter of the `SamplesPassedQuery`.
    pub samples_passed_query: Option<SamplesQueryParam<'a>>,

    /// If set, the time it took for the GPU to execute this draw command is added to the total
    /// stored inside the `TimeElapsedQuery`.
    #[cfg(any(safegl_desktop, safegl_webgl))]
    #[cfg_attr(docsrs, doc(cfg(any(safegl_desktop, safegl_webgl))))]
    pub time_elapsed_query: Option<&'a TimeElapsedQuery>,

    /// The queries that count the primitives that this draw generates, each of another vertex
    /// stream (see `PrimitivesGeneratedQuery::with_stream`), or the draw returns
    /// `DrawError::WrongQueryOperation`. The default is none.
    #[cfg(any(safegl_desktop, safegl_gles32))]
    #[cfg_attr(docsrs, doc(cfg(any(safegl_desktop, safegl_gles32))))]
    pub primitives_generated_queries: &'a [&'a PrimitivesGeneratedQuery],

    /// The queries that count the primitives that transform feedback writes during this draw,
    /// each of another vertex stream, or the draw returns `DrawError::WrongQueryOperation`. The
    /// default is none.
    pub transform_feedback_primitives_written_queries:
        &'a [&'a TransformFeedbackPrimitivesWrittenQuery],

    /// The pipeline statistics queries that count this draw, each of another statistic, or the
    /// draw returns `DrawError::WrongQueryOperation`. The default is none.
    #[cfg(safegl_gl46)]
    #[cfg_attr(docsrs, doc(cfg(safegl_gl46)))]
    pub pipeline_statistics_queries: &'a [&'a PipelineStatisticsQuery],

    /// If set, the query tells whether transform feedback overflowed during this draw.
    #[cfg(safegl_gl46)]
    #[cfg_attr(docsrs, doc(cfg(safegl_gl46)))]
    pub transform_feedback_overflow_query: Option<&'a TransformFeedbackOverflowQuery>,

    /// If set, the commands will only be executed if the specified query contains `true` or
    /// a number different than 0.
    ///
    /// OpenGL ES and WebGL need `GL_NV_conditional_render`, otherwise
    /// `DrawError::ConditionalRenderingNotSupported` is returned.
    #[cfg(safegl_desktop)]
    #[cfg_attr(docsrs, doc(cfg(safegl_desktop)))]
    pub condition: Option<ConditionalRendering<'a>>,

    /// If set, the generated primitives are captured in the buffers of the transform feedback
    /// object, after those that it captured before. Without it, the capture of the object that
    /// recorded during the previous draw is paused.
    pub transform_feedback: Option<&'a TransformFeedback<'a>>,

    /// If set, then the generated primitives will be smoothed.
    ///
    /// Note that blending needs to be enabled for this to work.
    #[cfg(safegl_desktop)]
    #[cfg_attr(docsrs, doc(cfg(safegl_desktop)))]
    pub smooth: Option<Smooth>,

    /// In your vertex shader or geometry shader, you have the possibility to mark some output
    /// varyings as `flat`. If this is the case, the value of one of the vertices will be used
    /// for the whole primitive. This variable allows you to specify which vertex.
    ///
    /// The default value is `LastVertex`, as this is the default in OpenGL. `FirstVertex` needs
    /// desktop OpenGL, `GL_EXT_provoking_vertex` or, on WebGL, `WEBGL_provoking_vertex`,
    /// otherwise it returns `DrawError::ProvokingVertexNotSupported`.
    #[cfg(any(safegl_desktop, safegl_webgl))]
    #[cfg_attr(docsrs, doc(cfg(any(safegl_desktop, safegl_webgl))))]
    pub provoking_vertex: ProvokingVertex,

    /// Hint for the GPU of the bounding box of the geometry.
    ///
    /// If you're using geometry shaders or tessellation shaders, it can be extremely advantageous
    /// for the GPU to know where on the screen the primitive is. This field specifies the
    /// bounding box (`x`, `y`, `z`, `w`) of the primitive and serves as a hint to the GPU.
    ///
    /// The GPU is free not to draw samples outside of the bounding box. Whether the samples are
    /// drawn is implementation-specific.
    ///
    /// This field is useless if you're not using a geometry shader or tessellation shader.
    ///
    /// Since this is purely an optimization, this parameter is ignored if the backend doesn't
    /// support it.
    #[cfg(safegl_gles32)]
    #[cfg_attr(docsrs, doc(cfg(safegl_gles32)))]
    pub primitive_bounding_box: (Range<f32>, Range<f32>, Range<f32>, Range<f32>),

    /// Whether indexed draws start a new primitive at an index value ("primitive restart"),
    /// to draw several strips or fans in one draw.
    ///
    /// WebGL 2 always restarts primitives at the maximum index value, so `Disabled` only differs
    /// from `FixedIndex` when the index buffer holds that value. Since the draw would restart
    /// anyway, it returns `DrawError::PrimitiveRestartAlwaysOn` there when the index buffer may
    /// hold it: when it was written with it, in the range drawn or not, or written in part, or
    /// copied from an index buffer of another type or that may hold it.
    pub primitive_restart: PrimitiveRestart,

    /// The tessellation levels of the patches of programs with a tessellation evaluation shader
    /// and no tessellation control shader, which only desktop OpenGL has. Ignored with a
    /// tessellation control shader, which writes the levels, and on OpenGL ES.
    #[cfg(safegl_desktop)]
    #[cfg_attr(docsrs, doc(cfg(safegl_desktop)))]
    pub patch_default_levels: PatchDefaultLevels,

    /// If enabled, shifts the depth value of towards of away from the camera. This is useful for
    /// drawing decals and wireframes, for example.
    ///
    /// OpenGL ES can only offset triangles, and WebGL lines too with `WEBGL_polygon_mode`:
    /// enabling `point` or `line` there returns `DrawError::PolygonOffsetNotSupported`.
    pub polygon_offset: PolygonOffset,

    /// Largest depth offset that `polygon_offset` may apply (`glPolygonOffsetClamp`), in the
    /// units of window-space depth, where the depth range spans 1. The default value is `0.0`,
    /// which doesn't clamp.
    ///
    /// A positive value limits the offset from above, a negative value from below. Shadow maps
    /// use it to keep the large slope-scaled offset (`PolygonOffset::factor`) that removes
    /// shadow acne from pushing surfaces seen at grazing angles so far back that shadows come
    /// loose from the objects that cast them ("peter-panning").
    ///
    /// Needs OpenGL 4.6, `GL_ARB_polygon_offset_clamp` or `GL_EXT_polygon_offset_clamp`
    /// (`EXT_polygon_offset_clamp` on WebGL). Otherwise a value other than `0.0` returns
    /// `DrawError::PolygonOffsetClampNotSupported`.
    #[cfg(any(safegl_gl46, safegl_webgl))]
    #[cfg_attr(docsrs, doc(cfg(any(safegl_gl46, safegl_webgl))))]
    pub polygon_offset_clamp: f32,

    /// Clip control origin. The default value is `LowerLeft`.
    ///
    /// Needs OpenGL 4.5, `GL_ARB_clip_control` or `GL_EXT_clip_control` (`EXT_clip_control` on
    /// WebGL), otherwise a value other than the default returns
    /// `DrawError::ClipControlNotSupported`, like for `clip_control_depth`.
    #[cfg(any(safegl_gl45, safegl_webgl))]
    #[cfg_attr(docsrs, doc(cfg(any(safegl_gl45, safegl_webgl))))]
    pub clip_control_origin: ClipControlOrigin,

    /// Clip control depth mode. The default value is `NegativeOneToOne`.
    #[cfg(any(safegl_gl45, safegl_webgl))]
    #[cfg_attr(docsrs, doc(cfg(any(safegl_gl45, safegl_webgl))))]
    pub clip_control_depth: ClipControlDepth,

    /// Index of the first instance of the draw. The default value is 0.
    ///
    /// The per-instance attributes start at this element of their buffers (added to the
    /// start of their slices), and the shader reads it in `gl_BaseInstance` (OpenGL 4.6 or
    /// `GL_ARB_shader_draw_parameters`), while `gl_InstanceID` still starts at 0. A draw without
    /// per-instance buffers draws one instance.
    ///
    /// Needs OpenGL 4.2, `GL_ARB_base_instance`, `GL_EXT_base_instance` or, on WebGL,
    /// `WEBGL_draw_instanced_base_vertex_base_instance`, otherwise a value other than 0 returns
    /// `DrawError::BaseInstanceNotSupported`.
    #[cfg(any(safegl_gl42, safegl_webgl))]
    #[cfg_attr(docsrs, doc(cfg(any(safegl_gl42, safegl_webgl))))]
    pub base_instance: u32,
}

/// Blending and color mask of one output of the fragment shader. See `DrawParameters::outputs`.
///
/// Example: the two outputs of weighted blended order-independent transparency, where the
/// first one adds up colors and the second one multiplies the transparencies.
///
/// ```
/// use safegl::draw_parameters::OutputState;
/// use safegl::{Blend, BlendingFunction, LinearBlendingFactor};
///
/// let add = BlendingFunction::Addition {
///     source: LinearBlendingFactor::One,
///     destination: LinearBlendingFactor::One,
/// };
/// let multiply = BlendingFunction::Addition {
///     source: LinearBlendingFactor::Zero,
///     destination: LinearBlendingFactor::OneMinusSourceColor,
/// };
/// let outputs = [
///     OutputState {
///         blend: Blend { color: add, alpha: add, ..Default::default() },
///         ..Default::default()
///     },
///     OutputState {
///         blend: Blend { color: multiply, alpha: multiply, ..Default::default() },
///         ..Default::default()
///     },
/// ];
/// let params = safegl::DrawParameters {
///     outputs: &outputs,
///     ..Default::default()
/// };
/// ```
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct OutputState {
    /// How the output is merged with the existing pixel. See `DrawParameters::blend`.
    pub blend: Blend,

    /// Which components of the output are written: red, green, blue, alpha. See
    /// `DrawParameters::color_mask`.
    pub color_mask: (bool, bool, bool, bool),
}

impl Default for OutputState {
    /// No blending and all components written, like the defaults of `DrawParameters`.
    fn default() -> OutputState {
        OutputState {
            blend: Default::default(),
            color_mask: (true, true, true, true),
        }
    }
}

/// Condition whether to render or not.
#[derive(Debug, Copy, Clone)]
#[cfg(safegl_desktop)]
#[cfg_attr(docsrs, doc(cfg(safegl_desktop)))]
pub struct ConditionalRendering<'a> {
    /// The query to use.
    pub query: ConditionQuery<'a>,

    /// If true, the GPU will wait until the query result has been obtained. If false, the GPU
    /// is free to ignore the query and draw anyway.
    pub wait: bool,

    /// If true, only samples that match those that were written with the query active will
    /// be drawn.
    pub per_region: bool,

    /// If true, the condition is inverted: the draw happens when the query found no samples, or
    /// no overflow (OpenGL 4.5 or `GL_ARB_conditional_render_inverted`, otherwise
    /// `DrawError::ConditionalRenderingInvertedNotSupported`).
    #[cfg(safegl_gl45)]
    #[cfg_attr(docsrs, doc(cfg(safegl_gl45)))]
    pub inverted: bool,
}

/// The query whose result conditions a draw.
#[cfg(safegl_desktop)]
#[cfg_attr(docsrs, doc(cfg(safegl_desktop)))]
#[derive(Debug, Copy, Clone)]
pub enum ConditionQuery<'a> {
    /// A `SamplesPassedQuery`: draws if samples passed.
    SamplesPassedQuery(&'a SamplesPassedQuery),
    /// An `AnySamplesPassedQuery`: draws if samples passed.
    AnySamplesPassedQuery(&'a AnySamplesPassedQuery),
    /// A `TransformFeedbackOverflowQuery`: draws if transform feedback overflowed.
    #[cfg(safegl_gl46)]
    #[cfg_attr(docsrs, doc(cfg(safegl_gl46)))]
    TransformFeedbackOverflowQuery(&'a TransformFeedbackOverflowQuery),
}

#[cfg(safegl_desktop)]
impl<'a> ConditionQuery<'a> {
    fn id(&self) -> glow::Query {
        match *self {
            ConditionQuery::SamplesPassedQuery(q) => q.get_id(),
            ConditionQuery::AnySamplesPassedQuery(q) => q.get_id(),
            #[cfg(safegl_gl46)]
            ConditionQuery::TransformFeedbackOverflowQuery(q) => q.get_id(),
        }
    }

    fn is_unused(&self) -> bool {
        match *self {
            ConditionQuery::SamplesPassedQuery(q) => q.is_unused(),
            ConditionQuery::AnySamplesPassedQuery(q) => q.is_unused(),
            #[cfg(safegl_gl46)]
            ConditionQuery::TransformFeedbackOverflowQuery(q) => q.is_unused(),
        }
    }

    fn get_context(&self) -> &Context {
        match *self {
            ConditionQuery::SamplesPassedQuery(q) => QueryExt::get_context(q),
            ConditionQuery::AnySamplesPassedQuery(q) => QueryExt::get_context(q),
            #[cfg(safegl_gl46)]
            ConditionQuery::TransformFeedbackOverflowQuery(q) => QueryExt::get_context(q),
        }
    }
}

#[cfg(safegl_desktop)]
impl<'a> From<&'a SamplesPassedQuery> for ConditionQuery<'a> {
    #[inline]
    fn from(query: &'a SamplesPassedQuery) -> ConditionQuery<'a> {
        ConditionQuery::SamplesPassedQuery(query)
    }
}

#[cfg(safegl_desktop)]
impl<'a> From<&'a AnySamplesPassedQuery> for ConditionQuery<'a> {
    #[inline]
    fn from(query: &'a AnySamplesPassedQuery) -> ConditionQuery<'a> {
        ConditionQuery::AnySamplesPassedQuery(query)
    }
}

#[cfg(safegl_gl46)]
impl<'a> From<&'a TransformFeedbackOverflowQuery> for ConditionQuery<'a> {
    #[inline]
    fn from(query: &'a TransformFeedbackOverflowQuery) -> ConditionQuery<'a> {
        ConditionQuery::TransformFeedbackOverflowQuery(query)
    }
}

#[cfg(safegl_desktop)]
impl<'a> From<SamplesQueryParam<'a>> for ConditionQuery<'a> {
    #[inline]
    fn from(query: SamplesQueryParam<'a>) -> ConditionQuery<'a> {
        match query {
            SamplesQueryParam::SamplesPassedQuery(q) => ConditionQuery::SamplesPassedQuery(q),
            SamplesQueryParam::AnySamplesPassedQuery(q) => ConditionQuery::AnySamplesPassedQuery(q),
        }
    }
}

/// The query to use for samples counting.
#[derive(Debug, Copy, Clone)]
pub enum SamplesQueryParam<'a> {
    /// A `SamplesPassedQuery`.
    #[cfg(safegl_desktop)]
    #[cfg_attr(docsrs, doc(cfg(safegl_desktop)))]
    SamplesPassedQuery(&'a SamplesPassedQuery),
    /// A `AnySamplesPassedQuery`.
    AnySamplesPassedQuery(&'a AnySamplesPassedQuery),
}

impl<'a> SamplesQueryParam<'a> {
    fn id(&self) -> glow::Query {
        match *self {
            #[cfg(safegl_desktop)]
            SamplesQueryParam::SamplesPassedQuery(q) => q.get_id(),
            SamplesQueryParam::AnySamplesPassedQuery(q) => q.get_id(),
        }
    }

    fn is_unused(&self) -> bool {
        match *self {
            #[cfg(safegl_desktop)]
            SamplesQueryParam::SamplesPassedQuery(q) => q.is_unused(),
            SamplesQueryParam::AnySamplesPassedQuery(q) => q.is_unused(),
        }
    }
}

/// Checks that the queries of the draw parameters belong to the current context.
pub(crate) fn check_query_contexts(
    ctxt: &context::CommandContext<'_>,
    draw_parameters: &DrawParameters<'_>,
) -> Result<(), DrawError> {
    let samples = |query: &SamplesQueryParam<'_>| match *query {
        #[cfg(safegl_desktop)]
        SamplesQueryParam::SamplesPassedQuery(q) => ctxt.owns(QueryExt::get_context(q)),
        SamplesQueryParam::AnySamplesPassedQuery(q) => ctxt.owns(QueryExt::get_context(q)),
    };

    let owned = draw_parameters
        .samples_passed_query
        .as_ref()
        .is_none_or(samples)
        && draw_parameters
            .transform_feedback_primitives_written_queries
            .iter()
            .all(|q| ctxt.owns(QueryExt::get_context(*q)));
    #[cfg(any(safegl_desktop, safegl_webgl))]
    let owned = owned
        && draw_parameters
            .time_elapsed_query
            .is_none_or(|q| ctxt.owns(QueryExt::get_context(q)));
    #[cfg(safegl_desktop)]
    let owned = owned
        && draw_parameters
            .condition
            .as_ref()
            .is_none_or(|condition| ctxt.owns(condition.query.get_context()));
    #[cfg(safegl_gl46)]
    let owned = owned
        && draw_parameters
            .pipeline_statistics_queries
            .iter()
            .all(|q| ctxt.owns(QueryExt::get_context(*q)))
        && draw_parameters
            .transform_feedback_overflow_query
            .is_none_or(|q| ctxt.owns(QueryExt::get_context(q)));
    #[cfg(any(safegl_desktop, safegl_gles32))]
    let owned = owned
        && draw_parameters
            .primitives_generated_queries
            .iter()
            .all(|q| ctxt.owns(QueryExt::get_context(*q)));

    if owned {
        Ok(())
    } else {
        Err(DrawError::ContextMismatch {
            object: "query".to_owned(),
        })
    }
}

/// Checks the queries of the draw parameters. Returns `Ok(false)` if the draw command must not
/// draw anything: its condition is a query that was never used, whose result counts as no
/// samples, and on which OpenGL refuses to begin conditional rendering.
#[cfg(not(safegl_desktop))]
pub(crate) fn check_queries(_: &DrawParameters<'_>) -> Result<bool, DrawError> {
    Ok(true)
}

/// Checks the queries of the draw parameters. Returns `Ok(false)` if the draw command must not
/// draw anything: its condition is a query that was never used, whose result counts as no
/// samples, and on which OpenGL refuses to begin conditional rendering.
#[cfg(safegl_desktop)]
pub(crate) fn check_queries(draw_parameters: &DrawParameters<'_>) -> Result<bool, DrawError> {
    let condition = match draw_parameters.condition {
        Some(ref condition) => condition,
        None => return Ok(true),
    };

    // a query can't count the samples of the draw that it conditions
    if let Some(ref samples) = draw_parameters.samples_passed_query {
        if samples.id() == condition.query.id() {
            return Err(DrawError::WrongQueryOperation);
        }
    }
    #[cfg(safegl_gl46)]
    if let Some(overflow) = draw_parameters.transform_feedback_overflow_query {
        if overflow.get_id() == condition.query.id() {
            return Err(DrawError::WrongQueryOperation);
        }
    }

    // a query that never began counts nothing: only an inverted condition draws
    #[cfg(safegl_gl45)]
    let inverted = condition.inverted;
    #[cfg(not(safegl_gl45))]
    let inverted = false;
    Ok(!condition.query.is_unused() || inverted)
}

#[cfg(safegl_desktop)]
impl<'a> From<&'a SamplesPassedQuery> for SamplesQueryParam<'a> {
    #[inline]
    fn from(r: &'a SamplesPassedQuery) -> SamplesQueryParam<'a> {
        SamplesQueryParam::SamplesPassedQuery(r)
    }
}

impl<'a> From<&'a AnySamplesPassedQuery> for SamplesQueryParam<'a> {
    #[inline]
    fn from(r: &'a AnySamplesPassedQuery) -> SamplesQueryParam<'a> {
        SamplesQueryParam::AnySamplesPassedQuery(r)
    }
}

/// Specifies the depth offset applied to rendered geometry
#[derive(Debug, Copy, Clone)]
pub struct PolygonOffset {
    /// Scale polygon depth with a factor
    pub factor: f32,
    /// Add a constant value to polygon depth
    pub units: f32,
    /// If true, the depth offset is enabled for points
    #[cfg(safegl_desktop)]
    #[cfg_attr(docsrs, doc(cfg(safegl_desktop)))]
    pub point: bool,
    /// If true, the depth offset is enabled for lines: on desktop OpenGL, and on WebGL with
    /// `WEBGL_polygon_mode`
    #[cfg(any(safegl_desktop, safegl_webgl))]
    #[cfg_attr(docsrs, doc(cfg(any(safegl_desktop, safegl_webgl))))]
    pub line: bool,
    /// If true, the depth offset is enabled for triangles
    pub fill: bool,
}

impl Default for PolygonOffset {
    fn default() -> Self {
        PolygonOffset {
            factor: 0.0,
            units: 0.0,
            #[cfg(safegl_desktop)]
            point: false,
            #[cfg(any(safegl_desktop, safegl_webgl))]
            line: false,
            fill: false,
        }
    }
}

/// Specifies the clip control origin.
#[cfg(any(safegl_gl45, safegl_webgl))]
#[cfg_attr(docsrs, doc(cfg(any(safegl_gl45, safegl_webgl))))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClipControlOrigin {
    /// The clip space origin is at the lower-left corner of the viewport. This is the default state.
    LowerLeft,

    /// The clip space origin is at the upper-left corner of the viewport. This will vertically invert the geometry inside the viewport.
    UpperLeft,
}

/// Specifies the clip control depth mode.
#[cfg(any(safegl_gl45, safegl_webgl))]
#[cfg_attr(docsrs, doc(cfg(any(safegl_gl45, safegl_webgl))))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClipControlDepth {
    /// The near and far clipping planes correspond to Z coordinates of -1 and +1. This is the default state.
    NegativeOneToOne,

    /// The near and far clipping planes correspond to Z coordinates of 0 and +1. This may improve numerical precision of depth mapping.
    ZeroToOne,
}

impl<'a> Default for DrawParameters<'a> {
    fn default() -> DrawParameters<'a> {
        DrawParameters {
            depth: Depth::default(),
            stencil: Default::default(),
            blend: Default::default(),
            color_mask: (true, true, true, true),
            outputs: &[],
            #[cfg(safegl_desktop)]
            logic_op: None,
            line_width: None,
            #[cfg(safegl_desktop)]
            point_size: None,
            #[cfg(safegl_desktop)]
            point_coord_origin: PointCoordOrigin::UpperLeft,
            #[cfg(safegl_desktop)]
            point_fade_threshold: 1.0,
            backface_culling: BackfaceCullingMode::CullingDisabled,
            front_face: FrontFace::CounterClockwise,
            #[cfg(any(safegl_desktop, safegl_webgl))]
            polygon_mode: PolygonMode::Fill,
            #[cfg(any(safegl_desktop, safegl_webgl))]
            clip_planes_bitmask: 0,
            multisample: Default::default(),
            dithering: true,
            viewport: None,
            scissor: None,
            #[cfg(safegl_gl41)]
            viewports: &[],
            draw_primitives: true,
            samples_passed_query: None,
            #[cfg(any(safegl_desktop, safegl_webgl))]
            time_elapsed_query: None,
            #[cfg(any(safegl_desktop, safegl_gles32))]
            primitives_generated_queries: &[],
            transform_feedback_primitives_written_queries: &[],
            #[cfg(safegl_gl46)]
            pipeline_statistics_queries: &[],
            #[cfg(safegl_gl46)]
            transform_feedback_overflow_query: None,
            #[cfg(safegl_desktop)]
            condition: None,
            transform_feedback: None,
            #[cfg(safegl_desktop)]
            smooth: None,
            #[cfg(any(safegl_desktop, safegl_webgl))]
            provoking_vertex: ProvokingVertex::LastVertex,
            #[cfg(safegl_gles32)]
            primitive_bounding_box: (-1.0..1.0, -1.0..1.0, -1.0..1.0, -1.0..1.0),
            primitive_restart: PrimitiveRestart::Disabled,
            #[cfg(safegl_desktop)]
            patch_default_levels: Default::default(),
            polygon_offset: Default::default(),
            #[cfg(any(safegl_gl46, safegl_webgl))]
            polygon_offset_clamp: 0.0,
            #[cfg(any(safegl_gl45, safegl_webgl))]
            clip_control_origin: ClipControlOrigin::LowerLeft,
            #[cfg(any(safegl_gl45, safegl_webgl))]
            clip_control_depth: ClipControlDepth::NegativeOneToOne,
            #[cfg(any(safegl_gl42, safegl_webgl))]
            base_instance: 0,
        }
    }
}

/// DEPRECATED. Checks parameters and returns an error if something is wrong.
pub fn validate(context: &Context, params: &DrawParameters<'_>) -> Result<(), DrawError> {
    if params.depth.range.0 < 0.0
        || params.depth.range.0 > 1.0
        || params.depth.range.1 < 0.0
        || params.depth.range.1 > 1.0
    {
        return Err(DrawError::InvalidDepthRange);
    }

    Ok(())
}

#[doc(hidden)]
///
/// `index_type` is the type of the indices of an indexed draw.
pub fn sync(
    ctxt: &mut context::CommandContext<'_>,
    draw_parameters: &DrawParameters<'_>,
    dimensions: (u32, u32),
    primitives_types: PrimitiveType,
    index_type: Option<crate::index::IndexType>,
) -> Result<(), DrawError> {
    if is_webgl() {
        validate_webgl(draw_parameters)?;
    }
    validate_outputs(ctxt, draw_parameters)?;
    depth::sync_depth(ctxt, &draw_parameters.depth)?;
    stencil::sync_stencil(ctxt, &draw_parameters.stencil, draw_parameters.front_face);
    let (blends, masks) = (
        draw_parameters.outputs.iter().map(|output| output.blend),
        draw_parameters
            .outputs
            .iter()
            .map(|output| output.color_mask),
    );
    #[cfg(safegl_desktop)]
    sync_logic_op(ctxt, draw_parameters)?;
    blend::sync_blending(ctxt, draw_parameters.blend, blends)?;
    sync_color_mask(ctxt, draw_parameters.color_mask, masks);
    sync_line_width(ctxt, draw_parameters.line_width)?;
    #[cfg(safegl_desktop)]
    sync_point_size(ctxt, draw_parameters.point_size);
    #[cfg(safegl_desktop)]
    sync_point_parameters(
        ctxt,
        draw_parameters.point_coord_origin,
        draw_parameters.point_fade_threshold,
    )?;
    sync_front_face(ctxt, draw_parameters.front_face);
    sync_backface_culling(ctxt, draw_parameters.backface_culling, draw_parameters.front_face);
    #[cfg(any(safegl_desktop, safegl_webgl))]
    sync_polygon_mode(ctxt, draw_parameters.polygon_mode)?;
    #[cfg(any(safegl_desktop, safegl_webgl))]
    sync_clip_planes_bitmask(ctxt, draw_parameters.clip_planes_bitmask)?;
    #[cfg(safegl_desktop)]
    sync_multisampling(ctxt, draw_parameters.multisample.enabled)?;
    sync_alpha_to_coverage(ctxt, draw_parameters.multisample.alpha_to_coverage);
    #[cfg(safegl_desktop)]
    sync_alpha_to_one(ctxt, draw_parameters.multisample.alpha_to_one)?;
    sync_sample_coverage(ctxt, draw_parameters.multisample.sample_coverage);
    #[cfg(any(safegl_desktop, safegl_gles32))]
    sync_sample_shading(ctxt, draw_parameters.multisample.sample_shading)?;
    #[cfg(any(safegl_desktop, safegl_gles31))]
    sync_sample_mask(ctxt, draw_parameters.multisample.sample_mask)?;
    sync_dithering(ctxt, draw_parameters.dithering);
    #[cfg(safegl_gl41)]
    let viewport_array = !draw_parameters.viewports.is_empty();
    #[cfg(not(safegl_gl41))]
    let viewport_array = false;
    if viewport_array {
        #[cfg(safegl_gl41)]
        sync_viewport_array(ctxt, draw_parameters, dimensions)?;
    } else {
        sync_viewport_scissor(
            ctxt,
            draw_parameters.viewport,
            draw_parameters.scissor,
            dimensions,
        );
    }
    sync_rasterizer_discard(ctxt, draw_parameters.draw_primitives)?;
    sync_queries(ctxt, draw_parameters)?;
    #[cfg(safegl_desktop)]
    sync_conditional_render(ctxt, draw_parameters.condition)?;
    #[cfg(safegl_desktop)]
    sync_smooth(ctxt, draw_parameters.smooth, primitives_types)?;
    #[cfg(any(safegl_desktop, safegl_webgl))]
    sync_provoking_vertex(ctxt, draw_parameters.provoking_vertex)?;
    #[cfg(safegl_gles32)]
    sync_primitive_bounding_box(ctxt, &draw_parameters.primitive_bounding_box);
    sync_primitive_restart(ctxt, draw_parameters.primitive_restart, index_type)?;
    #[cfg(safegl_desktop)]
    sync_patch_default_levels(ctxt, draw_parameters.patch_default_levels);
    sync_polygon_offset(
        ctxt,
        draw_parameters.polygon_offset,
        polygon_offset_clamp(draw_parameters),
    )?;
    #[cfg(any(safegl_gl45, safegl_webgl))]
    sync_clip_control(
        ctxt,
        draw_parameters.clip_control_origin,
        draw_parameters.clip_control_depth,
    )?;

    Ok(())
}

/// Returns true if the draw blends, in an output at least.
pub(crate) fn blends(draw_parameters: &DrawParameters<'_>) -> bool {
    let blends = |blend: &Blend| {
        blend.color != BlendingFunction::AlwaysReplace
            || blend.alpha != BlendingFunction::AlwaysReplace
    };
    let outputs = draw_parameters
        .outputs
        .iter()
        .any(|output| blends(&output.blend));
    blends(&draw_parameters.blend) || outputs
}

/// Checks the parameters that WebGL refuses where OpenGL ES accepts them, before anything is
/// synchronized: WebGL would raise `GL_INVALID_OPERATION` and draw nothing.
fn validate_webgl(draw_parameters: &DrawParameters<'_>) -> Result<(), DrawError> {
    let (near, far) = draw_parameters.depth.range;
    if near > far {
        return Err(DrawError::InvalidDepthRange);
    }
    stencil::validate_webgl_faces(&draw_parameters.stencil)?;
    blend::validate_webgl_constants(&draw_parameters.blend)
}

/// Checks that `DrawParameters::outputs` fits in the draw buffers, and that the backend can
/// give draw buffers their own state if an entry needs it.
fn validate_outputs(
    ctxt: &context::CommandContext<'_>,
    draw_parameters: &DrawParameters<'_>,
) -> Result<(), DrawError> {
    let outputs = draw_parameters.outputs;
    if outputs.is_empty() {
        return Ok(());
    }

    if outputs.len() > ctxt.capabilities.max_draw_buffers.max(0) as usize {
        return Err(DrawError::OutputStateIndexOutOfBounds);
    }

    let global_blend = blend::BlendState::new(draw_parameters.blend);
    let overrides = outputs.iter().any(|output| {
        output.color_mask != draw_parameters.color_mask
            || blend::BlendState::new(output.blend) != global_blend
    });
    if overrides && !indexed::is_supported(ctxt) {
        return Err(DrawError::PerOutputStateNotSupported);
    }

    Ok(())
}

/// Checks that the backend can blend with the second color output of the fragment shader if the
/// draw does, and that the framebuffer bound for drawing doesn't have more draw buffers than it
/// can then write. `draw_buffers` returns the number of draw buffers of that framebuffer.
pub(crate) fn validate_dual_source_blending(
    ctxt: &mut context::CommandContext<'_>,
    draw_parameters: &DrawParameters<'_>,
    draw_buffers: impl FnOnce(&mut context::CommandContext<'_>) -> usize,
) -> Result<(), DrawError> {
    let outputs = draw_parameters
        .outputs
        .iter()
        .any(|output| output.blend.uses_second_source());
    if !outputs && !draw_parameters.blend.uses_second_source() {
        return Ok(());
    }

    let max = ctxt
        .capabilities
        .limits
        .max_dual_source_draw_buffers
        .ok_or(DrawError::DualSourceBlendingNotSupported)?;
    if draw_buffers(ctxt) > max.max(0) as usize {
        return Err(DrawError::DualSourceBlendingTooManyDrawBuffers);
    }
    Ok(())
}

/// Sets the logic operation, after checking that the draw doesn't blend.
#[cfg(safegl_desktop)]
fn sync_logic_op(
    ctxt: &mut context::CommandContext<'_>,
    draw_parameters: &DrawParameters<'_>,
) -> Result<(), DrawError> {
    let logic_op = draw_parameters.logic_op;
    if let Some(logic_op) = logic_op {
        if ctxt.version.0 != Api::Gl {
            return Err(DrawError::LogicOpNotSupported);
        }
        let blends = draw_parameters.blend.is_enabled()
            || draw_parameters
                .outputs
                .iter()
                .any(|output| output.blend.is_enabled());
        if blends {
            return Err(DrawError::LogicOpWithBlending);
        }
        let logic_op = logic_op.to_glenum();
        if ctxt.state.logic_op != logic_op {
            unsafe { ctxt.extra.LogicOp(logic_op) };
            ctxt.state.logic_op = logic_op;
        }
    }
    if ctxt.version.0 == Api::Gl && ctxt.state.enabled_color_logic_op != logic_op.is_some() {
        ctxt.state.enabled_color_logic_op = logic_op.is_some();
        set_flag_enabled(ctxt, gl::COLOR_LOGIC_OP, logic_op.is_some());
    }
    Ok(())
}

/// Sets the color mask of all draw buffers to `mask`, then overrides it for the draw buffers
/// whose mask in `outputs` differs.
fn sync_color_mask(
    ctxt: &mut context::CommandContext<'_>,
    mask: (bool, bool, bool, bool),
    outputs: impl Iterator<Item = (bool, bool, bool, bool)>,
) {
    let gl_mask = (
        if mask.0 { 1 } else { 0 },
        if mask.1 { 1 } else { 0 },
        if mask.2 { 1 } else { 0 },
        if mask.3 { 1 } else { 0 },
    );

    // `glColorMask` sets every draw buffer, which also undoes the masks of a previous draw
    // call that had overrides
    if ctxt.state.indexed_color_mask || ctxt.state.color_mask != gl_mask {
        unsafe {
            ctxt.glow.color_mask(mask.0, mask.1, mask.2, mask.3);
        }

        ctxt.state.color_mask = gl_mask;
    }

    let mut indexed = false;
    for (index, m) in outputs.enumerate() {
        if m != mask {
            unsafe { indexed::color_mask(ctxt, index as u32, m) };
            indexed = true;
        }
    }
    ctxt.state.indexed_color_mask = indexed;
}

fn sync_line_width(
    ctxt: &mut context::CommandContext<'_>,
    line_width: Option<f32>,
) -> Result<(), DrawError> {
    if let Some(line_width) = line_width {
        // forward-compatible desktop contexts have no wide lines
        if !(line_width > 0.0) || (ctxt.version.0 == Api::Gl && line_width > 1.0) {
            return Err(DrawError::LineWidthNotSupported);
        }

        if ctxt.state.line_width != line_width {
            unsafe {
                ctxt.glow.line_width(line_width);
                ctxt.state.line_width = line_width;
            }
        }
    }

    Ok(())
}

#[cfg(safegl_desktop)]
#[cfg(safegl_desktop)]
fn sync_point_parameters(
    ctxt: &mut context::CommandContext<'_>,
    origin: PointCoordOrigin,
    fade_threshold: f32,
) -> Result<(), DrawError> {
    if ctxt.version.0 != Api::Gl {
        return match origin {
            PointCoordOrigin::UpperLeft => Ok(()),
            PointCoordOrigin::LowerLeft => Err(DrawError::PointCoordOriginNotSupported),
        };
    }

    let origin = match origin {
        PointCoordOrigin::UpperLeft => gl::UPPER_LEFT,
        PointCoordOrigin::LowerLeft => gl::LOWER_LEFT,
    };
    if ctxt.state.point_coord_origin != origin {
        unsafe {
            ctxt.extra
                .PointParameteri(gl::POINT_SPRITE_COORD_ORIGIN, origin as gl::types::GLint)
        };
        ctxt.state.point_coord_origin = origin;
    }
    // also turns NaN into 0
    let fade_threshold = fade_threshold.max(0.0);
    if ctxt.state.point_fade_threshold != fade_threshold {
        unsafe {
            ctxt.extra
                .PointParameterf(gl::POINT_FADE_THRESHOLD_SIZE, fade_threshold)
        };
        ctxt.state.point_fade_threshold = fade_threshold;
    }
    Ok(())
}

fn sync_point_size(ctxt: &mut context::CommandContext<'_>, point_size: Option<f32>) {
    // `glPointSize` doesn't exist on OpenGL ES and WebGL, where the size of points is the value
    // that the vertex shader writes to `gl_PointSize`
    if ctxt.version.0 != Api::Gl {
        return;
    }

    if let Some(point_size) = point_size {
        if ctxt.state.point_size != point_size {
            unsafe {
                ctxt.extra.PointSize(point_size);
                ctxt.state.point_size = point_size;
            }
        }
    }
}

/// The winding of the triangles that face the camera (`glFrontFace`).
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub enum FrontFace {
    /// Counter-clockwise triangles face the camera (`GL_CCW`), the default of OpenGL.
    #[default]
    CounterClockwise,
    /// Clockwise triangles face the camera (`GL_CW`), as in Direct3D.
    Clockwise,
}

fn sync_front_face(ctxt: &mut context::CommandContext<'_>, front_face: FrontFace) {
    let mode = match front_face {
        FrontFace::CounterClockwise => gl::CCW,
        FrontFace::Clockwise => gl::CW,
    };
    if ctxt.state.front_face != mode {
        unsafe { ctxt.glow.front_face(mode) };
        ctxt.state.front_face = mode;
    }
}

fn sync_backface_culling(
    ctxt: &mut context::CommandContext<'_>,
    backface_culling: BackfaceCullingMode,
    front_face: FrontFace,
) {
    // the culled faces are named by their winding, which faces the camera or not depending on
    // `glFrontFace`
    let backface_culling = match (backface_culling, front_face) {
        (BackfaceCullingMode::CullCounterClockwise, FrontFace::Clockwise) => {
            BackfaceCullingMode::CullClockwise
        }
        (BackfaceCullingMode::CullClockwise, FrontFace::Clockwise) => {
            BackfaceCullingMode::CullCounterClockwise
        }
        (mode, _) => mode,
    };
    match backface_culling {
        BackfaceCullingMode::CullingDisabled => unsafe {
            if ctxt.state.enabled_cull_face {
                ctxt.glow.disable(glow::CULL_FACE);
                ctxt.state.enabled_cull_face = false;
            }
        },
        BackfaceCullingMode::CullCounterClockwise => unsafe {
            if !ctxt.state.enabled_cull_face {
                ctxt.glow.enable(glow::CULL_FACE);
                ctxt.state.enabled_cull_face = true;
            }
            if ctxt.state.cull_face != gl::FRONT {
                ctxt.glow.cull_face(glow::FRONT);
                ctxt.state.cull_face = gl::FRONT;
            }
        },
        BackfaceCullingMode::CullClockwise => unsafe {
            if !ctxt.state.enabled_cull_face {
                ctxt.glow.enable(glow::CULL_FACE);
                ctxt.state.enabled_cull_face = true;
            }
            if ctxt.state.cull_face != gl::BACK {
                ctxt.glow.cull_face(glow::BACK);
                ctxt.state.cull_face = gl::BACK;
            }
        },
    }
}

#[cfg(any(safegl_desktop, safegl_webgl))]
fn sync_polygon_mode(
    ctxt: &mut context::CommandContext<'_>,
    polygon_mode: PolygonMode,
) -> Result<(), DrawError> {
    // `glPolygonMode` doesn't exist on OpenGL ES, where polygons are always filled; WebGL draws
    // their edges with `WEBGL_polygon_mode`, which has no points
    let desktop = ctxt.version.0 == Api::Gl;
    let webgl = ctxt.extensions.webgl_polygon_mode;
    let supported = match polygon_mode {
        PolygonMode::Fill => true,
        PolygonMode::Line => desktop || webgl,
        PolygonMode::Point => desktop,
    };
    if !supported {
        return Err(DrawError::PolygonModeNotSupported);
    }

    let polygon_mode = polygon_mode.to_glenum();
    if ctxt.state.polygon_mode != polygon_mode {
        if desktop {
            unsafe { ctxt.glow.polygon_mode(glow::FRONT_AND_BACK, polygon_mode) };
        } else {
            // the modes of `WEBGL_polygon_mode` have the values of OpenGL
            #[cfg(safegl_browser)]
            ctxt.webgl.polygon_mode(polygon_mode);
        }
        ctxt.state.polygon_mode = polygon_mode;
    }

    Ok(())
}

#[cfg(any(safegl_desktop, safegl_webgl))]
fn sync_clip_planes_bitmask(
    ctxt: &mut context::CommandContext<'_>,
    clip_planes_bitmask: u32,
) -> Result<(), DrawError> {
    // the limit is only queried when the planes change
    if clip_planes_bitmask == ctxt.state.enabled_clip_planes {
        return Ok(());
    }
    unsafe {
        // OpenGL ES has clip distances with `GL_EXT_clip_cull_distance`, and WebGL with
        // `WEBGL_clip_cull_distance`, whose names have the same values as the core ones
        let max_clip_planes: gl::types::GLint = if ctxt.version.0 == Api::Gl
            || ctxt.extensions.gl_ext_clip_cull_distance
            || ctxt.extensions.webgl_clip_cull_distance
        {
            ctxt.glow.get_parameter_i32(glow::MAX_CLIP_DISTANCES)
        } else {
            0
        };
        for i in 0..32 {
            if clip_planes_bitmask & (1 << i) != ctxt.state.enabled_clip_planes & (1 << i) {
                if clip_planes_bitmask & (1 << i) != 0 {
                    if i < max_clip_planes {
                        ctxt.glow.enable(glow::CLIP_DISTANCE0 + i as u32);
                        ctxt.state.enabled_clip_planes |= 1 << i;
                    } else {
                        return Err(DrawError::ClipPlaneIndexOutOfBounds);
                    }
                } else if i < max_clip_planes {
                    ctxt.glow.disable(glow::CLIP_DISTANCE0 + i as u32);
                    ctxt.state.enabled_clip_planes &= !(1 << i);
                }
            }
        }
        Ok(())
    }
}

#[cfg(safegl_desktop)]
fn sync_multisampling(
    ctxt: &mut context::CommandContext<'_>,
    multisampling: bool,
) -> Result<(), DrawError> {
    // `GL_MULTISAMPLE` doesn't exist on OpenGL ES and WebGL: multisampling is always enabled
    // when the framebuffer is multisampled
    if ctxt.version.0 != Api::Gl {
        return if multisampling {
            Ok(())
        } else {
            Err(DrawError::MultisamplingNotSupported)
        };
    }

    if ctxt.state.enabled_multisample != multisampling {
        unsafe {
            if multisampling {
                ctxt.glow.enable(glow::MULTISAMPLE);
                ctxt.state.enabled_multisample = true;
            } else {
                ctxt.glow.disable(glow::MULTISAMPLE);
                ctxt.state.enabled_multisample = false;
            }
        }
    }

    Ok(())
}

/// The multisampling of a draw (`DrawParameters::multisample`).
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct MultisampleState {
    /// Whether multisample antialiasing (MSAA) should be used. Default value is `true`.
    ///
    /// Note that you will need to set the appropriate option when creating the window.
    /// The recommended way to do is to leave this to `true`, and adjust the option when
    /// creating the window.
    ///
    /// OpenGL ES and WebGL can't disable multisampling of a multisampled framebuffer; `false`
    /// returns `DrawError::MultisamplingNotSupported`.
    #[cfg(safegl_desktop)]
    #[cfg_attr(docsrs, doc(cfg(safegl_desktop)))]
    pub enabled: bool,

    /// Whether the alpha of the first color output decides how many samples of each pixel the
    /// fragment covers (`GL_SAMPLE_ALPHA_TO_COVERAGE`). Default value is `false`.
    ///
    /// An alpha of 0.5 covers about half of the samples, so after the multisample resolve the
    /// fragment looks half transparent, without sorting and with depth writes intact. This is the
    /// usual way to anti-alias alpha-tested foliage, fences and hair. The exact pattern of
    /// samples is chosen by the implementation. It has no effect on a framebuffer that isn't
    /// multisampled.
    pub alpha_to_coverage: bool,

    /// Minimum fraction of the samples of each pixel that get their own run of the fragment
    /// shader (`glMinSampleShading`). Default value is `None`, which shades once per pixel.
    ///
    /// `Some(1.0)` shades every sample. This anti-aliases what the fragment shader computes
    /// (texture lookups, the alpha used by `alpha_to_coverage`, specular highlights) and not
    /// only the edges of triangles, at the cost of running the shader once per sample. Values are
    /// clamped to `0.0..=1.0`. It has no effect on a framebuffer that isn't multisampled.
    ///
    /// Needs OpenGL 4.0, OpenGL ES 3.2 or `GL_OES_sample_shading`. Otherwise, and on WebGL,
    /// returns `DrawError::SampleShadingNotSupported`.
    #[cfg(any(safegl_desktop, safegl_gles32))]
    #[cfg_attr(docsrs, doc(cfg(any(safegl_desktop, safegl_gles32))))]
    pub sample_shading: Option<f32>,

    /// Bitmask of the samples of each pixel that drawing may write (`glSampleMaski`). Bit `i`
    /// stands for sample `i`. Default value is `None`, which doesn't mask any sample.
    ///
    /// The mask is combined with the coverage of the primitive (and with `alpha_to_coverage`):
    /// a sample is written only if it is covered and its bit is set. Only the first 32 samples
    /// can be masked. It has no effect on a framebuffer that isn't multisampled.
    ///
    /// Needs OpenGL 3.2 or OpenGL ES 3.1. Otherwise, and on WebGL, returns
    /// `DrawError::SampleMaskNotSupported`.
    #[cfg(any(safegl_desktop, safegl_gles31))]
    #[cfg_attr(docsrs, doc(cfg(any(safegl_desktop, safegl_gles31))))]
    pub sample_mask: Option<u32>,

    /// Whether the alpha of the color outputs becomes 1 after it chose the covered samples
    /// (`GL_SAMPLE_ALPHA_TO_ONE`), so that `alpha_to_coverage` makes the fragments opaque instead
    /// of also blending them. Default value is `false`.
    ///
    /// Only desktop OpenGL has it: OpenGL ES returns `DrawError::AlphaToOneNotSupported`.
    #[cfg(safegl_desktop)]
    #[cfg_attr(docsrs, doc(cfg(safegl_desktop)))]
    pub alpha_to_one: bool,

    /// A fraction of the samples of each pixel that the fragments cover, combined with their
    /// coverage (`glSampleCoverage`). Default value is `None`, which doesn't change the coverage.
    pub sample_coverage: Option<SampleCoverage>,
}

impl Default for MultisampleState {
    fn default() -> MultisampleState {
        MultisampleState {
            #[cfg(safegl_desktop)]
            enabled: true,
            alpha_to_coverage: false,
            #[cfg(safegl_desktop)]
            alpha_to_one: false,
            sample_coverage: None,
            #[cfg(any(safegl_desktop, safegl_gles32))]
            sample_shading: None,
            #[cfg(any(safegl_desktop, safegl_gles31))]
            sample_mask: None,
        }
    }
}

/// The coverage given by `MultisampleState::sample_coverage`.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct SampleCoverage {
    /// The fraction of the samples, from 0 to 1.
    pub value: f32,
    /// Whether the fragments cover the other samples instead.
    pub invert: bool,
}

#[cfg(safegl_desktop)]
fn sync_alpha_to_one(
    ctxt: &mut context::CommandContext<'_>,
    alpha_to_one: bool,
) -> Result<(), DrawError> {
    if ctxt.version.0 != Api::Gl {
        return if alpha_to_one {
            Err(DrawError::AlphaToOneNotSupported)
        } else {
            Ok(())
        };
    }
    if ctxt.state.enabled_sample_alpha_to_one != alpha_to_one {
        ctxt.state.enabled_sample_alpha_to_one = alpha_to_one;
        set_flag_enabled(ctxt, gl::SAMPLE_ALPHA_TO_ONE, alpha_to_one);
    }
    Ok(())
}

fn sync_sample_coverage(
    ctxt: &mut context::CommandContext<'_>,
    sample_coverage: Option<SampleCoverage>,
) {
    if ctxt.state.enabled_sample_coverage != sample_coverage.is_some() {
        ctxt.state.enabled_sample_coverage = sample_coverage.is_some();
        set_flag_enabled(ctxt, gl::SAMPLE_COVERAGE, sample_coverage.is_some());
    }
    if let Some(SampleCoverage { value, invert }) = sample_coverage {
        let value = value.clamp(0.0, 1.0);
        if ctxt.state.sample_coverage != (value, invert) {
            unsafe { ctxt.glow.sample_coverage(value, invert) };
            ctxt.state.sample_coverage = (value, invert);
        }
    }
}

fn sync_alpha_to_coverage(ctxt: &mut context::CommandContext<'_>, alpha_to_coverage: bool) {
    if ctxt.state.enabled_sample_alpha_to_coverage != alpha_to_coverage {
        ctxt.state.enabled_sample_alpha_to_coverage = alpha_to_coverage;
        set_flag_enabled(ctxt, gl::SAMPLE_ALPHA_TO_COVERAGE, alpha_to_coverage);
    }
}

#[cfg(any(safegl_desktop, safegl_gles32))]
fn sync_sample_shading(
    ctxt: &mut context::CommandContext<'_>,
    sample_shading: Option<f32>,
) -> Result<(), DrawError> {
    let value = match sample_shading {
        Some(value) => value.clamp(0.0, 1.0),
        None => {
            // nothing to do if it was never enabled, which is always the case when unsupported
            if ctxt.state.enabled_sample_shading {
                ctxt.state.enabled_sample_shading = false;
                set_flag_enabled(ctxt, gl::SAMPLE_SHADING, false);
            }
            return Ok(());
        }
    };

    let core = ctxt.version.0 == Api::Gl || ctxt.version >= &Version(Api::GlEs, 3, 2);
    if !core && !ctxt.extensions.gl_oes_sample_shading {
        return Err(DrawError::SampleShadingNotSupported);
    }

    if ctxt.state.min_sample_shading != value {
        unsafe {
            if core {
                ctxt.extra.MinSampleShading(value);
            } else {
                ctxt.extra.MinSampleShadingOES(value);
            }
        }
        ctxt.state.min_sample_shading = value;
    }

    if !ctxt.state.enabled_sample_shading {
        ctxt.state.enabled_sample_shading = true;
        set_flag_enabled(ctxt, gl::SAMPLE_SHADING, true);
    }

    Ok(())
}

#[cfg(any(safegl_desktop, safegl_gles31))]
fn sync_sample_mask(
    ctxt: &mut context::CommandContext<'_>,
    sample_mask: Option<u32>,
) -> Result<(), DrawError> {
    let mask = match sample_mask {
        Some(mask) => mask,
        None => {
            // nothing to do if it was never enabled, which is always the case when unsupported
            if ctxt.state.enabled_sample_mask {
                ctxt.state.enabled_sample_mask = false;
                set_flag_enabled(ctxt, gl::SAMPLE_MASK, false);
            }
            return Ok(());
        }
    };

    if ctxt.version.0 != Api::Gl && ctxt.version < &Version(Api::GlEs, 3, 1) {
        return Err(DrawError::SampleMaskNotSupported);
    }

    if ctxt.state.sample_mask != mask {
        unsafe {
            ctxt.extra.SampleMaski(0, mask);
        }
        ctxt.state.sample_mask = mask;
    }

    if !ctxt.state.enabled_sample_mask {
        ctxt.state.enabled_sample_mask = true;
        set_flag_enabled(ctxt, gl::SAMPLE_MASK, true);
    }

    Ok(())
}

fn sync_dithering(ctxt: &mut context::CommandContext<'_>, dithering: bool) {
    if ctxt.state.enabled_dither != dithering {
        unsafe {
            if dithering {
                ctxt.glow.enable(glow::DITHER);
                ctxt.state.enabled_dither = true;
            } else {
                ctxt.glow.disable(glow::DITHER);
                ctxt.state.enabled_dither = false;
            }
        }
    }
}

/// One viewport of `DrawParameters::viewports`.
#[derive(Copy, Clone, Debug, PartialEq)]
#[cfg(safegl_gl41)]
#[cfg_attr(docsrs, doc(cfg(safegl_gl41)))]
pub struct ViewportState {
    /// The left of the viewport, in pixels. Fractions are kept, with
    /// `Limits::viewport_subpixel_bits` bits of precision.
    pub left: f32,
    /// The bottom of the viewport, in pixels.
    pub bottom: f32,
    /// The width of the viewport, in pixels. Negative widths return
    /// `DrawError::InvalidViewport`.
    pub width: f32,
    /// The height of the viewport, in pixels. Negative heights return
    /// `DrawError::InvalidViewport`.
    pub height: f32,
    /// The near and far depth values of the viewport, clamped to `0.0..=1.0`, like
    /// `Depth::range`.
    pub depth_range: (f32, f32),
    /// The scissor box of the viewport, if any.
    pub scissor: Option<Rect>,
}

#[cfg(safegl_gl41)]
impl From<Rect> for ViewportState {
    /// The viewport `rect`, with the `(0.0, 1.0)` depth range and no scissor box.
    fn from(rect: Rect) -> ViewportState {
        ViewportState {
            left: rect.left as f32,
            bottom: rect.bottom as f32,
            width: rect.width as f32,
            height: rect.height as f32,
            depth_range: (0.0, 1.0),
            scissor: None,
        }
    }
}

/// Sets the viewports of `DrawParameters::viewports`, which isn't empty.
#[cfg(safegl_gl41)]
fn sync_viewport_array(
    ctxt: &mut context::CommandContext<'_>,
    draw_parameters: &DrawParameters<'_>,
    surface_dimensions: (u32, u32),
) -> Result<(), DrawError> {
    let viewports = draw_parameters.viewports;
    let max = ctxt
        .capabilities
        .limits
        .max_viewports
        .ok_or(DrawError::ViewportArrayNotSupported)?
        .max(0) as usize;
    if viewports.len() > max {
        return Err(DrawError::ViewportIndexOutOfBounds);
    }
    if draw_parameters.viewport.is_some()
        || draw_parameters.scissor.is_some()
        || draw_parameters.depth.range != (0.0, 1.0)
    {
        return Err(DrawError::ViewportArrayConflict);
    }
    let valid = |v: &ViewportState| {
        v.left.is_finite() && v.bottom.is_finite() && v.width >= 0.0 && v.height >= 0.0
    };
    if !viewports.iter().all(valid) {
        return Err(DrawError::InvalidViewport);
    }

    let whole = ViewportState::from(Rect {
        left: 0,
        bottom: 0,
        width: surface_dimensions.0,
        height: surface_dimensions.1,
    });
    let entries = (0..max)
        .map(|i| {
            let v = viewports.get(i).unwrap_or(&whole);
            // the largest viewport of the implementation, like `glViewport` does
            let width = v.width.min(ctxt.capabilities.max_viewport_dims.0 as f32);
            let height = v.height.min(ctxt.capabilities.max_viewport_dims.1 as f32);
            let (near, far) = v.depth_range;
            let scissor = v.scissor.map(|s| {
                [
                    s.left as gl::types::GLint,
                    s.bottom as gl::types::GLint,
                    s.width as gl::types::GLsizei,
                    s.height as gl::types::GLsizei,
                ]
            });
            (
                [v.left, v.bottom, width, height],
                [near.clamp(0.0, 1.0) as f64, far.clamp(0.0, 1.0) as f64],
                scissor,
            )
        })
        .collect::<Vec<_>>();

    // clears and restores call the functions that set every index, so the arrays aren't cached
    let rects = entries.iter().map(|e| e.0).collect::<Vec<_>>();
    let depth_ranges = entries.iter().map(|e| e.1).collect::<Vec<_>>();
    let scissors = entries
        .iter()
        .map(|e| e.2.unwrap_or([0, 0, 0, 0]))
        .collect::<Vec<_>>();
    unsafe {
        ctxt.glow.viewport_f32_slice(0, max as i32, &rects);
        ctxt.glow.depth_range_f64_slice(0, max as i32, &depth_ranges);
        ctxt.glow.scissor_slice(0, max as i32, &scissors);
        for (i, entry) in entries.iter().enumerate() {
            if entry.2.is_some() {
                ctxt.glow.enable_draw_buffer(glow::SCISSOR_TEST, i as u32);
            } else {
                ctxt.glow.disable_draw_buffer(glow::SCISSOR_TEST, i as u32);
            }
        }
    }

    // the caches of the single viewport, depth range and scissor box describe index 0, which
    // clears use
    let (rect, depth_range, scissor) = entries[0];
    ctxt.state.viewport = if rect.iter().all(|c| c.fract() == 0.0) {
        Some((rect[0] as i32, rect[1] as i32, rect[2] as i32, rect[3] as i32))
    } else {
        None
    };
    ctxt.state.depth_range = (depth_range[0] as f32, depth_range[1] as f32);
    ctxt.state.scissor = Some(scissors[0].into());
    ctxt.state.enabled_scissor_test = scissor.is_some();
    ctxt.state.viewport_array_dirty = true;
    Ok(())
}

/// Gives every viewport index the viewport, depth range and scissor state of index 0 again after
/// a draw with `DrawParameters::viewports`, before the single viewport is set.
fn reset_viewport_array(ctxt: &mut context::CommandContext<'_>) {
    if !ctxt.state.viewport_array_dirty {
        return;
    }
    ctxt.state.viewport_array_dirty = false;
    // `glViewport`, `glScissor`, `glDepthRange` and `glEnable` set every index
    ctxt.state.viewport = None;
    ctxt.state.scissor = None;
    let (near, far) = ctxt.state.depth_range;
    unsafe {
        ctxt.glow.depth_range_f64(near as f64, far as f64);
        if ctxt.state.enabled_scissor_test {
            ctxt.glow.enable(glow::SCISSOR_TEST);
        } else {
            ctxt.glow.disable(glow::SCISSOR_TEST);
        }
    }
}

fn sync_viewport_scissor(
    ctxt: &mut context::CommandContext<'_>,
    viewport: Option<Rect>,
    scissor: Option<Rect>,
    surface_dimensions: (u32, u32),
) {
    reset_viewport_array(ctxt);
    // viewport
    if let Some(viewport) = viewport {
        assert!(
            viewport.width <= ctxt.capabilities.max_viewport_dims.0 as u32,
            "Viewport dimensions are too large"
        );
        assert!(
            viewport.height <= ctxt.capabilities.max_viewport_dims.1 as u32,
            "Viewport dimensions are too large"
        );

        let viewport = (
            viewport.left as gl::types::GLint,
            viewport.bottom as gl::types::GLint,
            viewport.width as gl::types::GLsizei,
            viewport.height as gl::types::GLsizei,
        );

        if ctxt.state.viewport != Some(viewport) {
            unsafe {
                ctxt.glow
                    .viewport(viewport.0, viewport.1, viewport.2, viewport.3);
            }
            ctxt.state.viewport = Some(viewport);
        }
    } else {
        assert!(
            surface_dimensions.0 <= ctxt.capabilities.max_viewport_dims.0 as u32,
            "Viewport dimensions are too large"
        );
        assert!(
            surface_dimensions.1 <= ctxt.capabilities.max_viewport_dims.1 as u32,
            "Viewport dimensions are too large"
        );

        let viewport = (
            0,
            0,
            surface_dimensions.0 as gl::types::GLsizei,
            surface_dimensions.1 as gl::types::GLsizei,
        );

        if ctxt.state.viewport != Some(viewport) {
            unsafe {
                ctxt.glow
                    .viewport(viewport.0, viewport.1, viewport.2, viewport.3);
            }
            ctxt.state.viewport = Some(viewport);
        }
    }

    // scissor
    if let Some(scissor) = scissor {
        let scissor = (
            scissor.left as gl::types::GLint,
            scissor.bottom as gl::types::GLint,
            scissor.width as gl::types::GLsizei,
            scissor.height as gl::types::GLsizei,
        );

        unsafe {
            if ctxt.state.scissor != Some(scissor) {
                ctxt.glow
                    .scissor(scissor.0, scissor.1, scissor.2, scissor.3);
                ctxt.state.scissor = Some(scissor);
            }

            if !ctxt.state.enabled_scissor_test {
                ctxt.glow.enable(glow::SCISSOR_TEST);
                ctxt.state.enabled_scissor_test = true;
            }
        }
    } else {
        unsafe {
            if ctxt.state.enabled_scissor_test {
                ctxt.glow.disable(glow::SCISSOR_TEST);
                ctxt.state.enabled_scissor_test = false;
            }
        }
    }
}

fn sync_rasterizer_discard(
    ctxt: &mut context::CommandContext<'_>,
    draw_primitives: bool,
) -> Result<(), DrawError> {
    // `GL_RASTERIZER_DISCARD` is core in OpenGL 3.0 and OpenGL ES 3.0
    if ctxt.state.enabled_rasterizer_discard == draw_primitives {
        if draw_primitives {
            unsafe {
                ctxt.glow.disable(glow::RASTERIZER_DISCARD);
            }
            ctxt.state.enabled_rasterizer_discard = false;
        } else {
            unsafe {
                ctxt.glow.enable(glow::RASTERIZER_DISCARD);
            }
            ctxt.state.enabled_rasterizer_discard = true;
        }
    }

    Ok(())
}

fn sync_queries(
    ctxt: &mut context::CommandContext<'_>,
    draw_parameters: &DrawParameters<'_>,
) -> Result<(), DrawError> {
    match draw_parameters.samples_passed_query {
        #[cfg(safegl_desktop)]
        Some(SamplesQueryParam::SamplesPassedQuery(q)) => q.begin_query(ctxt)?,
        Some(SamplesQueryParam::AnySamplesPassedQuery(q)) => q.begin_query(ctxt)?,
        None => AnySamplesPassedQuery::end_samples_passed_query(ctxt),
    }

    #[cfg(any(safegl_desktop, safegl_webgl))]
    if let Some(time_elapsed_query) = draw_parameters.time_elapsed_query {
        time_elapsed_query.begin_query(ctxt)?;
    } else {
        AnySamplesPassedQuery::end_time_elapsed_query(ctxt);
    }

    // the queries of primitives, statistics and overflows, by target and stream
    #[cfg_attr(not(any(safegl_desktop, safegl_gles32)), allow(unused_mut))]
    let mut queries: smallvec::SmallVec<[&RawQuery; 4]> = draw_parameters
        .transform_feedback_primitives_written_queries
        .iter()
        .map(|q| q.raw())
        .collect();
    #[cfg(any(safegl_desktop, safegl_gles32))]
    queries.extend(
        draw_parameters
            .primitives_generated_queries
            .iter()
            .map(|q| q.raw()),
    );
    #[cfg(safegl_gl46)]
    {
        queries.extend(
            draw_parameters
                .pipeline_statistics_queries
                .iter()
                .map(|q| q.raw()),
        );
        queries.extend(draw_parameters.transform_feedback_overflow_query.map(|q| q.raw()));
    }
    sync_indexed_queries(ctxt, &queries, false)?;

    Ok(())
}

#[cfg(safegl_desktop)]
fn sync_conditional_render(
    ctxt: &mut context::CommandContext<'_>,
    condition: Option<ConditionalRendering<'_>>,
) -> Result<(), DrawError> {
    if let Some(condition) = condition {
        let ConditionalRendering {
            query,
            wait,
            per_region,
            ..
        } = condition;
        #[cfg(safegl_gl45)]
        let inverted = condition.inverted;
        #[cfg(not(safegl_gl45))]
        let inverted = false;

        // the condition of a query that never began is known: the draw happens unconditionally,
        // since `check_queries` only lets it through if the condition is inverted
        if query.is_unused() {
            #[cfg(safegl_gl45)]
            if inverted
                && !(ctxt.version >= &Version(Api::Gl, 4, 5)
                    || ctxt.extensions.gl_arb_conditional_render_inverted)
            {
                return Err(DrawError::ConditionalRenderingInvertedNotSupported);
            }
            AnySamplesPassedQuery::end_conditional_render(ctxt);
            return Ok(());
        }

        match query {
            ConditionQuery::SamplesPassedQuery(q) => {
                q.begin_conditional_render(ctxt, wait, per_region, inverted)
            }
            ConditionQuery::AnySamplesPassedQuery(q) => {
                q.begin_conditional_render(ctxt, wait, per_region, inverted)
            }
            #[cfg(safegl_gl46)]
            ConditionQuery::TransformFeedbackOverflowQuery(q) => {
                q.begin_conditional_render(ctxt, wait, per_region, inverted)
            }
        }
    } else {
        AnySamplesPassedQuery::end_conditional_render(ctxt);
        Ok(())
    }
}

#[cfg(safegl_desktop)]
fn sync_smooth(
    ctxt: &mut context::CommandContext<'_>,
    smooth: Option<Smooth>,
    primitive_type: PrimitiveType,
) -> Result<(), DrawError> {
    if let Some(smooth) = smooth {
        // check if smoothing is supported, it isn't on OpenGL ES
        if ctxt.version.0 != Api::Gl {
            return Err(DrawError::SmoothingNotSupported);
        }

        let hint = smooth.to_glenum();

        match primitive_type {
            // point
            PrimitiveType::Points => return Err(DrawError::SmoothingNotSupported),

            // line
            PrimitiveType::LinesList
            | PrimitiveType::LinesListAdjacency
            | PrimitiveType::LineStrip
            | PrimitiveType::LineStripAdjacency
            | PrimitiveType::LineLoop => unsafe {
                if !ctxt.state.enabled_line_smooth {
                    ctxt.state.enabled_line_smooth = true;
                    ctxt.glow.enable(glow::LINE_SMOOTH);
                }

                if ctxt.state.smooth.0 != hint {
                    ctxt.state.smooth.0 = hint;
                    ctxt.glow.hint(glow::LINE_SMOOTH_HINT, hint);
                }
            },

            // polygon
            _ => unsafe {
                if !ctxt.state.enabled_polygon_smooth {
                    ctxt.state.enabled_polygon_smooth = true;
                    ctxt.glow.enable(glow::POLYGON_SMOOTH);
                }

                if ctxt.state.smooth.1 != hint {
                    ctxt.state.smooth.1 = hint;
                    ctxt.glow.hint(glow::POLYGON_SMOOTH_HINT, hint);
                }
            },
        }
    } else {
        match primitive_type {
            // point
            PrimitiveType::Points => (),

            // line
            PrimitiveType::LinesList
            | PrimitiveType::LinesListAdjacency
            | PrimitiveType::LineStrip
            | PrimitiveType::LineStripAdjacency
            | PrimitiveType::LineLoop => unsafe {
                if ctxt.state.enabled_line_smooth {
                    ctxt.state.enabled_line_smooth = false;
                    ctxt.glow.disable(glow::LINE_SMOOTH);
                }
            },

            // polygon
            _ => unsafe {
                if ctxt.state.enabled_polygon_smooth {
                    ctxt.state.enabled_polygon_smooth = false;
                    ctxt.glow.disable(glow::POLYGON_SMOOTH);
                }
            },
        }
    }

    Ok(())
}

#[cfg(any(safegl_desktop, safegl_webgl))]
fn sync_provoking_vertex(
    ctxt: &mut context::CommandContext<'_>,
    value: ProvokingVertex,
) -> Result<(), DrawError> {
    let value = match value {
        ProvokingVertex::LastVertex => gl::LAST_VERTEX_CONVENTION,
        ProvokingVertex::FirstVertex => gl::FIRST_VERTEX_CONVENTION,
    };

    if ctxt.state.provoking_vertex == value {
        return Ok(());
    }

    if ctxt.version.0 == Api::Gl {
        unsafe {
            ctxt.extra.ProvokingVertex(value);
        }
        ctxt.state.provoking_vertex = value;
    } else if ctxt.extensions.gl_ext_provoking_vertex {
        unsafe {
            ctxt.extra.ProvokingVertexEXT(value);
        }
        ctxt.state.provoking_vertex = value;
    } else if ctxt.extensions.webgl_provoking_vertex {
        // the conventions of `WEBGL_provoking_vertex` have the values of OpenGL
        #[cfg(safegl_browser)]
        ctxt.webgl.provoking_vertex(value);
        ctxt.state.provoking_vertex = value;
    } else {
        return Err(DrawError::ProvokingVertexNotSupported);
    }

    Ok(())
}

#[cfg(safegl_gles32)]
fn sync_primitive_bounding_box(
    ctxt: &mut context::CommandContext<'_>,
    bb: &(Range<f32>, Range<f32>, Range<f32>, Range<f32>),
) {
    let value = (
        bb.0.start, bb.1.start, bb.2.start, bb.3.start, bb.0.end, bb.1.end, bb.2.end, bb.3.end,
    );

    if ctxt.state.primitive_bounding_box == value {
        return;
    }

    if ctxt.version >= &Version(Api::GlEs, 3, 2) {
        unsafe {
            ctxt.extra.PrimitiveBoundingBox(
                value.0, value.1, value.2, value.3, value.4, value.5, value.6, value.7,
            );
        }
        ctxt.state.primitive_bounding_box = value;
    } else if ctxt.extensions.gl_arb_es3_2_compatibility {
        unsafe {
            ctxt.extra.PrimitiveBoundingBoxARB(
                value.0, value.1, value.2, value.3, value.4, value.5, value.6, value.7,
            );
        }
        ctxt.state.primitive_bounding_box = value;
    } else if ctxt.extensions.gl_oes_primitive_bounding_box {
        unsafe {
            ctxt.extra.PrimitiveBoundingBoxOES(
                value.0, value.1, value.2, value.3, value.4, value.5, value.6, value.7,
            );
        }
        ctxt.state.primitive_bounding_box = value;
    } else if ctxt.extensions.gl_ext_primitive_bounding_box {
        unsafe {
            ctxt.extra.PrimitiveBoundingBoxEXT(
                value.0, value.1, value.2, value.3, value.4, value.5, value.6, value.7,
            );
        }
        ctxt.state.primitive_bounding_box = value;
    }
}

fn sync_primitive_restart(
    ctxt: &mut context::CommandContext<'_>,
    restart: PrimitiveRestart,
    index_type: Option<crate::index::IndexType>,
) -> Result<(), DrawError> {
    // WebGL 2 always restarts primitives at the maximum index value and rejects
    // GL_PRIMITIVE_RESTART_FIXED_INDEX, so there is nothing to change
    if is_webgl() {
        return Ok(());
    }

    // `GL_PRIMITIVE_RESTART_FIXED_INDEX` is core in OpenGL ES 3.0 but only in OpenGL 4.3; below,
    // desktop OpenGL restarts at the maximum value of the index type with `GL_PRIMITIVE_RESTART`
    let fixed_index = ctxt.version.0 == Api::GlEs
        || ctxt.version >= &Version(Api::Gl, 4, 3)
        || ctxt.extensions.gl_arb_es3_compatibility;
    let (fixed, index) = match restart {
        PrimitiveRestart::Disabled => (false, None),
        PrimitiveRestart::FixedIndex if fixed_index => (true, None),
        PrimitiveRestart::FixedIndex => (
            false,
            Some(match index_type {
                Some(crate::index::IndexType::U8) => u8::MAX as u32,
                Some(crate::index::IndexType::U16) => u16::MAX as u32,
                Some(crate::index::IndexType::U32) | None => u32::MAX,
            }),
        ),
        #[cfg(safegl_desktop)]
        PrimitiveRestart::Index(index) => {
            if ctxt.version.0 != Api::Gl {
                return Err(DrawError::PrimitiveRestartIndexNotSupported);
            }
            (false, Some(index))
        }
    };

    if fixed_index && ctxt.state.enabled_primitive_fixed_restart != fixed {
        unsafe {
            if fixed {
                ctxt.glow.enable(glow::PRIMITIVE_RESTART_FIXED_INDEX);
            } else {
                ctxt.glow.disable(glow::PRIMITIVE_RESTART_FIXED_INDEX);
            }
        }
        ctxt.state.enabled_primitive_fixed_restart = fixed;
    }

    // `GL_PRIMITIVE_RESTART` only exists on desktop OpenGL
    if ctxt.version.0 == Api::Gl {
        if ctxt.state.enabled_primitive_restart != index.is_some() {
            unsafe {
                if index.is_some() {
                    ctxt.glow.enable(glow::PRIMITIVE_RESTART);
                } else {
                    ctxt.glow.disable(glow::PRIMITIVE_RESTART);
                }
            }
            ctxt.state.enabled_primitive_restart = index.is_some();
        }
        if let Some(index) = index {
            if ctxt.state.primitive_restart_index != index {
                unsafe { ctxt.extra.PrimitiveRestartIndex(index) };
                ctxt.state.primitive_restart_index = index;
            }
        }
    }

    Ok(())
}

/// The tessellation levels of patches without a tessellation control shader
/// (`GL_PATCH_DEFAULT_OUTER_LEVEL` and `GL_PATCH_DEFAULT_INNER_LEVEL`). OpenGL clamps them to 1
/// and `Limits::max_tess_gen_level`.
#[cfg(safegl_desktop)]
#[cfg_attr(docsrs, doc(cfg(safegl_desktop)))]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct PatchDefaultLevels {
    /// The levels of the edges (`gl_TessLevelOuter`).
    pub outer: [f32; 4],
    /// The levels of the inside (`gl_TessLevelInner`).
    pub inner: [f32; 2],
}

#[cfg(safegl_desktop)]
impl Default for PatchDefaultLevels {
    /// The levels of OpenGL: 1 everywhere.
    fn default() -> PatchDefaultLevels {
        PatchDefaultLevels {
            outer: [1.0; 4],
            inner: [1.0; 2],
        }
    }
}

#[cfg(safegl_desktop)]
fn sync_patch_default_levels(ctxt: &mut context::CommandContext<'_>, levels: PatchDefaultLevels) {
    // the programs of OpenGL ES always have a tessellation control shader
    if ctxt.version.0 != Api::Gl {
        return;
    }
    unsafe {
        if ctxt.state.patch_default_outer_level != levels.outer {
            ctxt.extra
                .PatchParameterfv(gl::PATCH_DEFAULT_OUTER_LEVEL, levels.outer.as_ptr());
            ctxt.state.patch_default_outer_level = levels.outer;
        }
        if ctxt.state.patch_default_inner_level != levels.inner {
            ctxt.extra
                .PatchParameterfv(gl::PATCH_DEFAULT_INNER_LEVEL, levels.inner.as_ptr());
            ctxt.state.patch_default_inner_level = levels.inner;
        }
    }
}

/// Whether and where indexed draws start a new primitive ("primitive restart").
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub enum PrimitiveRestart {
    /// Indices are never special.
    #[default]
    Disabled,

    /// A new primitive starts at the largest value of the index type: `u8::MAX`, `u16::MAX` or
    /// `u32::MAX` (`GL_PRIMITIVE_RESTART_FIXED_INDEX`).
    FixedIndex,

    /// A new primitive starts at this index value (`glPrimitiveRestartIndex`). Only desktop
    /// OpenGL has it: OpenGL ES returns `DrawError::PrimitiveRestartIndexNotSupported`.
    #[cfg(safegl_desktop)]
    #[cfg_attr(docsrs, doc(cfg(safegl_desktop)))]
    Index(u32),
}

fn set_flag_enabled(ctxt: &mut context::CommandContext<'_>, cap: gl::types::GLenum, enabled: bool) {
    if enabled {
        unsafe {
            ctxt.glow.enable(cap);
        }
    } else {
        unsafe {
            ctxt.glow.disable(cap);
        }
    }
}

#[cfg(any(safegl_gl46, safegl_webgl))]
fn polygon_offset_clamp(draw_parameters: &DrawParameters<'_>) -> f32 {
    draw_parameters.polygon_offset_clamp
}

#[cfg(not(any(safegl_gl46, safegl_webgl)))]
fn polygon_offset_clamp(_: &DrawParameters<'_>) -> f32 {
    0.0
}

fn sync_polygon_offset(
    ctxt: &mut context::CommandContext<'_>,
    offset: PolygonOffset,
    clamp: f32,
) -> Result<(), DrawError> {
    sync_polygon_offset_point_line(ctxt, offset)?;

    if ctxt.state.polygon_offset != (offset.factor, offset.units)
        || ctxt.state.polygon_offset_clamp != clamp
    {
        if clamp == 0.0 && ctxt.state.polygon_offset_clamp == 0.0 {
            unsafe {
                ctxt.glow.polygon_offset(offset.factor, offset.units);
            }
        } else {
            set_polygon_offset_clamp(ctxt, offset.factor, offset.units, clamp)?;
        }
        ctxt.state.polygon_offset = (offset.factor, offset.units);
        ctxt.state.polygon_offset_clamp = clamp;
    }

    if offset.fill != ctxt.state.enabled_polygon_offset_fill {
        ctxt.state.enabled_polygon_offset_fill = offset.fill;
        set_flag_enabled(ctxt, gl::POLYGON_OFFSET_FILL, offset.fill);
    }

    Ok(())
}

fn sync_polygon_offset_point_line(
    ctxt: &mut context::CommandContext<'_>,
    offset: PolygonOffset,
) -> Result<(), DrawError> {
    #[cfg(safegl_desktop)]
    let point = offset.point;
    #[cfg(not(safegl_desktop))]
    let point = false;
    #[cfg(any(safegl_desktop, safegl_webgl))]
    let line = offset.line;
    #[cfg(not(any(safegl_desktop, safegl_webgl)))]
    let line = false;

    // `GL_POLYGON_OFFSET_POINT` and `GL_POLYGON_OFFSET_LINE` don't exist on OpenGL ES; WebGL has
    // `GL_POLYGON_OFFSET_LINE` with `WEBGL_polygon_mode`
    if ctxt.version.0 != Api::Gl {
        let line_offsets = ctxt.extensions.webgl_polygon_mode;
        #[cfg(any(safegl_desktop, safegl_webgl))]
        if point || (line && !line_offsets) {
            return Err(DrawError::PolygonOffsetNotSupported);
        }
        if line_offsets && line != ctxt.state.enabled_polygon_offset_line {
            ctxt.state.enabled_polygon_offset_line = line;
            set_flag_enabled(ctxt, gl::POLYGON_OFFSET_LINE, line);
        }
        return Ok(());
    }

    if point != ctxt.state.enabled_polygon_offset_point {
        ctxt.state.enabled_polygon_offset_point = point;
        set_flag_enabled(ctxt, gl::POLYGON_OFFSET_POINT, point);
    }

    if line != ctxt.state.enabled_polygon_offset_line {
        ctxt.state.enabled_polygon_offset_line = line;
        set_flag_enabled(ctxt, gl::POLYGON_OFFSET_LINE, line);
    }

    Ok(())
}

#[cfg(any(safegl_gl46, safegl_webgl))]
fn set_polygon_offset_clamp(
    ctxt: &mut context::CommandContext<'_>,
    factor: f32,
    units: f32,
    clamp: f32,
) -> Result<(), DrawError> {
    unsafe {
        if ctxt.version >= &Version(Api::Gl, 4, 6) || ctxt.extensions.gl_arb_polygon_offset_clamp
        {
            ctxt.extra.PolygonOffsetClamp(factor, units, clamp);
        } else if ctxt.extensions.gl_ext_polygon_offset_clamp {
            #[cfg(safegl_browser)]
            ctxt.webgl.polygon_offset_clamp(factor, units, clamp);
            #[cfg(not(safegl_browser))]
            ctxt.extra.PolygonOffsetClampEXT(factor, units, clamp);
        } else {
            return Err(DrawError::PolygonOffsetClampNotSupported);
        }
    }
    Ok(())
}

// below the `gl46` level the clamp is always 0
#[cfg(not(any(safegl_gl46, safegl_webgl)))]
fn set_polygon_offset_clamp(
    _: &mut context::CommandContext<'_>,
    _: f32,
    _: f32,
    _: f32,
) -> Result<(), DrawError> {
    unreachable!()
}

#[cfg(any(safegl_gl45, safegl_webgl))]
fn sync_clip_control(
    ctxt: &mut context::CommandContext<'_>,
    origin: ClipControlOrigin,
    depth: ClipControlDepth,
) -> Result<(), DrawError> {
    let origin = match origin {
        ClipControlOrigin::LowerLeft => gl::LOWER_LEFT,
        ClipControlOrigin::UpperLeft => gl::UPPER_LEFT,
    };
    let depth = match depth {
        ClipControlDepth::NegativeOneToOne => gl::NEGATIVE_ONE_TO_ONE,
        ClipControlDepth::ZeroToOne => gl::ZERO_TO_ONE,
    };

    if ctxt.state.clip_control == (origin, depth) {
        return Ok(());
    }

    if !unsafe { clip_control(ctxt, origin, depth) } {
        return Err(DrawError::ClipControlNotSupported);
    }
    ctxt.state.clip_control = (origin, depth);
    Ok(())
}

/// Calls `glClipControl` (OpenGL 4.5, `GL_ARB_clip_control`) or `glClipControlEXT`
/// (`GL_EXT_clip_control`, `EXT_clip_control` on WebGL). Returns false if the backend has
/// neither.
#[cfg(any(safegl_gl45, safegl_webgl))]
pub(crate) unsafe fn clip_control(
    ctxt: &context::CommandContext<'_>,
    origin: gl::types::GLenum,
    depth: gl::types::GLenum,
) -> bool {
    if ctxt.version >= &Version(Api::Gl, 4, 5) || ctxt.extensions.gl_arb_clip_control {
        ctxt.extra.ClipControl(origin, depth);
    } else if ctxt.extensions.gl_ext_clip_control {
        #[cfg(safegl_browser)]
        ctxt.webgl.clip_control(origin, depth);
        #[cfg(not(safegl_browser))]
        ctxt.extra.ClipControlEXT(origin, depth);
    } else {
        return false;
    }
    true
}
