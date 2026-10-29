use crate::context::CommandContext;

use crate::gl;
use crate::DrawError;
use glow::HasContext;
use smallvec::SmallVec;

/// Blend effect that the GPU will use for blending.
///
/// Blending happens at the end of the rendering process, when the GPU wants to write the
/// pixels over pixels that already exist in the framebuffer. The blending function allows
/// you to choose how it should merge the two.
///
/// If you want to add transparent objects one over another, use
/// `Blend::alpha_blending()`.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Blend {
    /// The blending function for color channels.
    pub color: BlendingFunction,
    /// The blending function for alpha channels.
    pub alpha: BlendingFunction,
    /// A constant color that can be used in the blending functions.
    pub constant_value: (f32, f32, f32, f32),
}

impl Blend {
    /// Returns a blend effect to add transparent objects over others.
    ///
    /// The colors are blended with `source × source alpha + destination × (1 - source alpha)`,
    /// and the alphas with `source + destination × (1 - source alpha)`, so that the alpha of the
    /// target is the coverage of everything drawn over it.
    pub fn alpha_blending() -> Blend {
        Blend {
            color: BlendingFunction::Addition {
                source: LinearBlendingFactor::SourceAlpha,
                destination: LinearBlendingFactor::OneMinusSourceAlpha,
            },
            alpha: BlendingFunction::Addition {
                source: LinearBlendingFactor::One,
                destination: LinearBlendingFactor::OneMinusSourceAlpha,
            },
            constant_value: (0.0, 0.0, 0.0, 0.0),
        }
    }

    /// Returns a blend effect that blends the colors and the alpha with an advanced equation.
    pub fn advanced(equation: AdvancedBlending) -> Blend {
        Blend {
            color: BlendingFunction::Advanced(equation),
            alpha: BlendingFunction::Advanced(equation),
            constant_value: (0.0, 0.0, 0.0, 0.0),
        }
    }
}

impl Blend {
    /// Whether drawing with it enables `GL_BLEND`.
    pub(crate) fn is_enabled(&self) -> bool {
        BlendState::new(*self).enabled
    }

    /// The advanced equation that it blends with, or `DrawError::AdvancedBlendingMismatch` if
    /// its color or its alpha function is advanced and the other one differs.
    pub(crate) fn advanced_equation(&self) -> Result<Option<AdvancedBlending>, DrawError> {
        match (self.color, self.alpha) {
            (BlendingFunction::Advanced(color), BlendingFunction::Advanced(alpha))
                if color == alpha =>
            {
                Ok(Some(color))
            }
            (BlendingFunction::Advanced(_), _) | (_, BlendingFunction::Advanced(_)) => {
                Err(DrawError::AdvancedBlendingMismatch)
            }
            _ => Ok(None),
        }
    }

    /// Whether it blends with the second color output of the fragment shader, which is
    /// dual-source blending.
    pub(crate) fn uses_second_source(&self) -> bool {
        let state = BlendState::new(*self);
        let (a, b, c, d) = state.func;
        state.enabled
            && [a, b, c, d].iter().any(|factor| {
                matches!(
                    *factor,
                    gl::SRC1_COLOR
                        | gl::ONE_MINUS_SRC1_COLOR
                        | gl::SRC1_ALPHA
                        | gl::ONE_MINUS_SRC1_ALPHA
                )
            })
    }
}

impl Default for Blend {
    fn default() -> Blend {
        Blend {
            color: BlendingFunction::AlwaysReplace,
            alpha: BlendingFunction::AlwaysReplace,
            constant_value: (1.0, 1.0, 1.0, 1.0),
        }
    }
}

/// Function that the GPU will use for blending.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum BlendingFunction {
    /// Simply overwrite the destination pixel with the source pixel.
    ///
    /// The alpha channels are simply ignored. This is the default mode.
    ///
    /// For example writing `(0.5, 0.9, 0.4, 0.2)` over `(0.9, 0.1, 0.4, 0.3)` will
    /// result in `(0.5, 0.9, 0.4, 0.2)`.
    AlwaysReplace,

    /// For each individual component (red, green, blue, and alpha), the minimum value is chosen
    /// between the source and the destination.
    ///
    /// For example writing `(0.5, 0.9, 0.4, 0.2)` over `(0.9, 0.1, 0.4, 0.3)` will
    /// result in `(0.5, 0.1, 0.4, 0.2)`.
    Min,

    /// For each individual component (red, green, blue, and alpha), the maximum value is chosen
    /// between the source and the destination.
    ///
    /// For example writing `(0.5, 0.9, 0.4, 0.2)` over `(0.9, 0.1, 0.4, 0.3)` will
    /// result in `(0.9, 0.9, 0.4, 0.3)`.
    Max,

    /// For each individual component (red, green, blue, and alpha), a weighted addition
    /// between the source and the destination.
    ///
    /// The result is equal to `source_component * source_factor + dest_component * dest_factor`,
    /// where `source_factor` and `dest_factor` are the values of `source` and `destination` of
    /// this enum.
    Addition {
        /// The factor to apply to the source pixel.
        source: LinearBlendingFactor,

        /// The factor to apply to the destination pixel.
        destination: LinearBlendingFactor,
    },

    /// For each individual component (red, green, blue, and alpha), a weighted subtraction
    /// of the source by the destination.
    ///
    /// The result is equal to `source_component * source_factor - dest_component * dest_factor`,
    /// where `source_factor` and `dest_factor` are the values of `source` and `destination` of
    /// this enum.
    Subtraction {
        /// The factor to apply to the source pixel.
        source: LinearBlendingFactor,

        /// The factor to apply to the destination pixel.
        destination: LinearBlendingFactor,
    },

    /// For each individual component (red, green, blue, and alpha), a weighted subtraction
    /// of the destination by the source.
    ///
    /// The result is equal to `-source_component * source_factor + dest_component * dest_factor`,
    /// where `source_factor` and `dest_factor` are the values of `source` and `destination` of
    /// this enum.
    ReverseSubtraction {
        /// The factor to apply to the source pixel.
        source: LinearBlendingFactor,

        /// The factor to apply to the destination pixel.
        destination: LinearBlendingFactor,
    },

    /// An advanced blend equation, which ignores the blend factors and blends the colors and the
    /// alpha together: the color and the alpha functions of a `Blend` are the same advanced
    /// equation, which `Blend::advanced` gives, or draws return
    /// `DrawError::AdvancedBlendingMismatch`. Draws with it return
    /// `DrawError::AdvancedBlendingNotSupported` without `GL_KHR_blend_equation_advanced` or
    /// OpenGL ES 3.2, and on WebGL.
    Advanced(AdvancedBlending),
}

/// Indicates which value to multiply each component with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinearBlendingFactor {
    /// Multiply the source or destination component by zero, which always
    /// gives `0.0`.
    Zero,

    /// Multiply the source or destination component by one, which always
    /// gives you the original value.
    One,

    /// Multiply the source or destination component by its corresponding value
    /// in the source.
    ///
    /// If you apply this to the source components, you get the values squared.
    SourceColor,

    /// Equivalent to `1 - SourceColor`.
    OneMinusSourceColor,

    /// Multiply the source or destination component by its corresponding value
    /// in the destination.
    ///
    /// If you apply this to the destination components, you get the values squared.
    DestinationColor,

    /// Equivalent to `1 - DestinationColor`.
    OneMinusDestinationColor,

    /// Multiply the source or destination component by the alpha value of the source.
    SourceAlpha,

    /// Multiply the source or destination component by the smallest value of
    /// `SourceAlpha` and `1 - DestinationAlpha`.
    SourceAlphaSaturate,

    /// Multiply the source or destination component by `1.0` minus the alpha value of the source.
    OneMinusSourceAlpha,

    /// Multiply the source or destination component by the alpha value of the destination.
    DestinationAlpha,

    /// Multiply the source or destination component by `1.0` minus the alpha value of the
    /// destination.
    OneMinusDestinationAlpha,

    /// Multiply the source or destination component by the corresponding value
    /// in `Blend::const_value`.
    ConstantColor,

    /// Multiply the source or destination component by `1.0` minus the corresponding
    /// value in `Blend::const_value`.
    OneMinusConstantColor,

    /// Multiply the source or destination component by the alpha value of `Blend::const_value`.
    ConstantAlpha,

    /// Multiply the source or destination component by `1.0` minus the alpha value of
    /// `Blend::const_value`.
    OneMinusConstantAlpha,

    /// Multiply the source or destination component by its corresponding value in the second
    /// color output of the fragment shader, the one declared with
    /// `layout(location = 0, index = 1)`. This is dual-source blending.
    ///
    /// It needs desktop OpenGL or `GL_EXT_blend_func_extended` (whose GLSL ES shaders declare
    /// `#extension GL_EXT_blend_func_extended : require`), and drawing into at most
    /// `Limits::max_dual_source_draw_buffers` color buffers, usually 1. Otherwise, and on
    /// WebGL, drawing returns `DrawError::DualSourceBlendingNotSupported` or
    /// `DrawError::DualSourceBlendingTooManyDrawBuffers`. These requirements apply to the four
    /// `*SourceOne*` factors.
    SourceOneColor,

    /// Equivalent to `1 - SourceOneColor`.
    OneMinusSourceOneColor,

    /// Multiply the source or destination component by the alpha value of source index 1.
    SourceOneAlpha,

    /// Multiply the source or destination component by `1.0` minus the alpha value of source index 1.
    OneMinusSourceOneAlpha,
}

impl LinearBlendingFactor {
    fn to_glenum(&self) -> gl::types::GLenum {
        match *self {
            LinearBlendingFactor::Zero => gl::ZERO,
            LinearBlendingFactor::One => gl::ONE,
            LinearBlendingFactor::SourceColor => gl::SRC_COLOR,
            LinearBlendingFactor::OneMinusSourceColor => gl::ONE_MINUS_SRC_COLOR,
            LinearBlendingFactor::DestinationColor => gl::DST_COLOR,
            LinearBlendingFactor::OneMinusDestinationColor => gl::ONE_MINUS_DST_COLOR,
            LinearBlendingFactor::SourceAlpha => gl::SRC_ALPHA,
            LinearBlendingFactor::OneMinusSourceAlpha => gl::ONE_MINUS_SRC_ALPHA,
            LinearBlendingFactor::DestinationAlpha => gl::DST_ALPHA,
            LinearBlendingFactor::OneMinusDestinationAlpha => gl::ONE_MINUS_DST_ALPHA,
            LinearBlendingFactor::SourceAlphaSaturate => gl::SRC_ALPHA_SATURATE,
            LinearBlendingFactor::ConstantColor => gl::CONSTANT_COLOR,
            LinearBlendingFactor::OneMinusConstantColor => gl::ONE_MINUS_CONSTANT_COLOR,
            LinearBlendingFactor::ConstantAlpha => gl::CONSTANT_ALPHA,
            LinearBlendingFactor::OneMinusConstantAlpha => gl::ONE_MINUS_CONSTANT_ALPHA,
            LinearBlendingFactor::SourceOneColor => gl::SRC1_COLOR,
            LinearBlendingFactor::OneMinusSourceOneColor => gl::ONE_MINUS_SRC1_COLOR,
            LinearBlendingFactor::SourceOneAlpha => gl::SRC1_ALPHA,
            LinearBlendingFactor::OneMinusSourceOneAlpha => gl::ONE_MINUS_SRC1_ALPHA,
        }
    }
}


/// An advanced blend equation (`GL_KHR_blend_equation_advanced`, core in OpenGL ES 3.2): one of
/// the equations of the PDF and SVG specifications, applied to the colors and the alpha, which
/// ignores the blend factors.
///
/// The fragment shader declares the equations that it is drawn with, like
/// `layout(blend_support_multiply) out;`, or `layout(blend_support_all_equations) out;` for all
/// of them. The colors are premultiplied by the alpha, and the draw writes a single color
/// output.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AdvancedBlending {
    /// `source × destination` (`GL_MULTIPLY_KHR`).
    Multiply,
    /// `source + destination - source × destination` (`GL_SCREEN_KHR`).
    Screen,
    /// `HardLight` with the source and the destination swapped (`GL_OVERLAY_KHR`).
    Overlay,
    /// The minimum of each component (`GL_DARKEN_KHR`).
    Darken,
    /// The maximum of each component (`GL_LIGHTEN_KHR`).
    Lighten,
    /// The destination brightened to reflect the source (`GL_COLORDODGE_KHR`).
    ColorDodge,
    /// The destination darkened to reflect the source (`GL_COLORBURN_KHR`).
    ColorBurn,
    /// `Multiply` or `Screen`, depending on the source (`GL_HARDLIGHT_KHR`).
    HardLight,
    /// A softer `HardLight` (`GL_SOFTLIGHT_KHR`).
    SoftLight,
    /// The absolute difference of the components (`GL_DIFFERENCE_KHR`).
    Difference,
    /// `Difference` with a lower contrast (`GL_EXCLUSION_KHR`).
    Exclusion,
    /// The hue of the source with the saturation and the luminosity of the destination
    /// (`GL_HSL_HUE_KHR`).
    HslHue,
    /// The saturation of the source with the hue and the luminosity of the destination
    /// (`GL_HSL_SATURATION_KHR`).
    HslSaturation,
    /// The hue and the saturation of the source with the luminosity of the destination
    /// (`GL_HSL_COLOR_KHR`).
    HslColor,
    /// The luminosity of the source with the hue and the saturation of the destination
    /// (`GL_HSL_LUMINOSITY_KHR`).
    HslLuminosity,
}

impl AdvancedBlending {
    /// Every advanced blend equation.
    pub const ALL: [AdvancedBlending; 15] = [
        AdvancedBlending::Multiply,
        AdvancedBlending::Screen,
        AdvancedBlending::Overlay,
        AdvancedBlending::Darken,
        AdvancedBlending::Lighten,
        AdvancedBlending::ColorDodge,
        AdvancedBlending::ColorBurn,
        AdvancedBlending::HardLight,
        AdvancedBlending::SoftLight,
        AdvancedBlending::Difference,
        AdvancedBlending::Exclusion,
        AdvancedBlending::HslHue,
        AdvancedBlending::HslSaturation,
        AdvancedBlending::HslColor,
        AdvancedBlending::HslLuminosity,
    ];

    fn to_glenum(self) -> gl::types::GLenum {
        match self {
            AdvancedBlending::Multiply => 0x9294,
            AdvancedBlending::Screen => 0x9295,
            AdvancedBlending::Overlay => 0x9296,
            AdvancedBlending::Darken => 0x9297,
            AdvancedBlending::Lighten => 0x9298,
            AdvancedBlending::ColorDodge => 0x9299,
            AdvancedBlending::ColorBurn => 0x929A,
            AdvancedBlending::HardLight => 0x929B,
            AdvancedBlending::SoftLight => 0x929C,
            AdvancedBlending::Difference => 0x929E,
            AdvancedBlending::Exclusion => 0x92A0,
            AdvancedBlending::HslHue => 0x92AD,
            AdvancedBlending::HslSaturation => 0x92AE,
            AdvancedBlending::HslColor => 0x92AF,
            AdvancedBlending::HslLuminosity => 0x92B0,
        }
    }

    /// The name of the equation in the `blend_support_*` layout qualifier of GLSL.
    fn layout_name(self) -> &'static str {
        match self {
            AdvancedBlending::Multiply => "multiply",
            AdvancedBlending::Screen => "screen",
            AdvancedBlending::Overlay => "overlay",
            AdvancedBlending::Darken => "darken",
            AdvancedBlending::Lighten => "lighten",
            AdvancedBlending::ColorDodge => "colordodge",
            AdvancedBlending::ColorBurn => "colorburn",
            AdvancedBlending::HardLight => "hardlight",
            AdvancedBlending::SoftLight => "softlight",
            AdvancedBlending::Difference => "difference",
            AdvancedBlending::Exclusion => "exclusion",
            AdvancedBlending::HslHue => "hsl_hue",
            AdvancedBlending::HslSaturation => "hsl_saturation",
            AdvancedBlending::HslColor => "hsl_color",
            AdvancedBlending::HslLuminosity => "hsl_luminosity",
        }
    }

    /// The bit of the equation in the masks of `blend_support_mask`.
    pub(crate) fn bit(self) -> u32 {
        1 << AdvancedBlending::ALL.iter().position(|&e| e == self).unwrap()
    }

    /// The mask of the equations that a `blend_support_*` layout qualifier of GLSL declares,
    /// `None` if `qualifier` isn't one.
    pub(crate) fn blend_support_mask(qualifier: &str) -> Option<u32> {
        let name = qualifier.strip_prefix("blend_support_")?;
        if name == "all_equations" {
            return Some((1 << AdvancedBlending::ALL.len()) - 1);
        }
        AdvancedBlending::ALL
            .iter()
            .find(|equation| equation.layout_name() == name)
            .map(|equation| equation.bit())
    }
}

/// The OpenGL state that a `Blend` translates to.
#[derive(Copy, Clone, Debug, PartialEq)]
pub(crate) struct BlendState {
    /// Whether `GL_BLEND` is enabled. When it isn't, the other fields keep their defaults.
    enabled: bool,
    /// Color and alpha equations.
    equation: (gl::types::GLenum, gl::types::GLenum),
    /// Source and destination factors of the color, then of the alpha.
    func: (
        gl::types::GLenum,
        gl::types::GLenum,
        gl::types::GLenum,
        gl::types::GLenum,
    ),
    /// The blend constant, if one of the factors uses it.
    constant: Option<(f32, f32, f32, f32)>,
}

impl BlendState {
    pub(crate) fn new(blend: Blend) -> BlendState {
        #[inline(always)]
        fn blend_eq(blending_function: BlendingFunction) -> gl::types::GLenum {
            match blending_function {
                BlendingFunction::AlwaysReplace | BlendingFunction::Addition { .. } => {
                    gl::FUNC_ADD
                }
                BlendingFunction::Subtraction { .. } => gl::FUNC_SUBTRACT,
                BlendingFunction::ReverseSubtraction { .. } => gl::FUNC_REVERSE_SUBTRACT,

                // `GL_MIN` and `GL_MAX` are core in OpenGL ES 3.0
                BlendingFunction::Min => gl::MIN,
                BlendingFunction::Max => gl::MAX,
                BlendingFunction::Advanced(equation) => equation.to_glenum(),
            }
        }

        #[inline(always)]
        fn blending_factors(
            blending_function: BlendingFunction,
        ) -> Option<(LinearBlendingFactor, LinearBlendingFactor)> {
            match blending_function {
                BlendingFunction::AlwaysReplace
                | BlendingFunction::Min
                | BlendingFunction::Max
                | BlendingFunction::Advanced(_) => None,
                BlendingFunction::Addition {
                    source,
                    destination,
                } => Some((source, destination)),
                BlendingFunction::Subtraction {
                    source,
                    destination,
                } => Some((source, destination)),
                BlendingFunction::ReverseSubtraction {
                    source,
                    destination,
                } => Some((source, destination)),
            }
        }

        if let (BlendingFunction::AlwaysReplace, BlendingFunction::AlwaysReplace) =
            (blend.color, blend.alpha)
        {
            // Both color and alpha always replace. This equals no blending.
            return BlendState {
                enabled: false,
                equation: (gl::FUNC_ADD, gl::FUNC_ADD),
                func: (gl::ONE, gl::ZERO, gl::ONE, gl::ZERO),
                constant: None,
            };
        }

        // Map to dummy factors if the blending equation does not use the factors.
        let (color_factor_src, color_factor_dst) = blending_factors(blend.color)
            .unwrap_or((LinearBlendingFactor::One, LinearBlendingFactor::Zero));
        let (alpha_factor_src, alpha_factor_dst) = blending_factors(blend.alpha)
            .unwrap_or((LinearBlendingFactor::One, LinearBlendingFactor::Zero));

        let uses_constant = [
            color_factor_src,
            color_factor_dst,
            alpha_factor_src,
            alpha_factor_dst,
        ]
        .iter()
        .any(|factor| {
            matches!(
                factor,
                LinearBlendingFactor::ConstantColor
                    | LinearBlendingFactor::OneMinusConstantColor
                    | LinearBlendingFactor::ConstantAlpha
                    | LinearBlendingFactor::OneMinusConstantAlpha
            )
        });

        BlendState {
            enabled: true,
            equation: (blend_eq(blend.color), blend_eq(blend.alpha)),
            func: (
                color_factor_src.to_glenum(),
                color_factor_dst.to_glenum(),
                alpha_factor_src.to_glenum(),
                alpha_factor_dst.to_glenum(),
            ),
            constant: if uses_constant {
                Some(blend.constant_value)
            } else {
                None
            },
        }
    }
}

/// Sets the blending of all draw buffers to `blend`, then overrides it for the draw buffers
/// whose blending in `outputs` differs.
///
/// The caller has already checked that `outputs` fits in the draw buffers and that the backend
/// supports per-draw-buffer state if any entry differs from `blend`.
/// Returns `DrawError::BlendConstantsMismatch` if the color function of `blend` has a constant
/// color factor and a constant alpha factor, which WebGL refuses (its `blendFuncSeparate` checks
/// the color factors only).
pub fn validate_webgl_constants(blend: &Blend) -> Result<(), DrawError> {
    let (source, destination) = match blend.color {
        BlendingFunction::Addition {
            source,
            destination,
        }
        | BlendingFunction::Subtraction {
            source,
            destination,
        }
        | BlendingFunction::ReverseSubtraction {
            source,
            destination,
        } => (source, destination),
        BlendingFunction::AlwaysReplace
        | BlendingFunction::Min
        | BlendingFunction::Max
        | BlendingFunction::Advanced(_) => return Ok(()),
    };
    let color = |factor| {
        matches!(
            factor,
            LinearBlendingFactor::ConstantColor | LinearBlendingFactor::OneMinusConstantColor
        )
    };
    let alpha = |factor| {
        matches!(
            factor,
            LinearBlendingFactor::ConstantAlpha | LinearBlendingFactor::OneMinusConstantAlpha
        )
    };
    if (color(source) && alpha(destination)) || (alpha(source) && color(destination)) {
        return Err(DrawError::BlendConstantsMismatch);
    }
    Ok(())
}

pub fn sync_blending(
    ctxt: &mut CommandContext<'_>,
    blend: Blend,
    outputs: impl Iterator<Item = Blend>,
) -> Result<(), DrawError> {
    let global = BlendState::new(blend);
    let overrides = outputs
        .map(BlendState::new)
        .enumerate()
        .filter(|(_, state)| *state != global)
        .collect::<SmallVec<[_; 8]>>();

    // There is a single blend constant for all draw buffers.
    let mut constant = global.constant;
    for (_, state) in &overrides {
        match (constant, state.constant) {
            (Some(a), Some(b)) if a != b => return Err(DrawError::BlendingParameterNotSupported),
            (None, Some(b)) => constant = Some(b),
            _ => (),
        }
    }
    if let Some(constant) = constant {
        if ctxt.state.blend_color != constant {
            let (r, g, b, a) = constant;
            unsafe {
                ctxt.glow.blend_color(r, g, b, a);
            }
            ctxt.state.blend_color = constant;
        }
    }

    // The global functions set every draw buffer. If a previous draw call overrode some of
    // them, the cache can't be trusted and everything is set again.
    let force = ctxt.state.indexed_blend;

    if force || ctxt.state.enabled_blend != global.enabled {
        unsafe {
            if global.enabled {
                ctxt.glow.enable(glow::BLEND);
            } else {
                ctxt.glow.disable(glow::BLEND);
            }
        }
        ctxt.state.enabled_blend = global.enabled;
    }

    // When blending is disabled, the cached equation and function are kept.
    let equation = if global.enabled {
        global.equation
    } else {
        ctxt.state.blend_equation
    };
    if force || ctxt.state.blend_equation != equation {
        unsafe {
            // the advanced equations set the colors and the alpha together
            if is_advanced(equation.0) {
                ctxt.glow.blend_equation(equation.0);
            } else {
                ctxt.glow.blend_equation_separate(equation.0, equation.1);
            }
        }
        ctxt.state.blend_equation = equation;
    }

    let func = if global.enabled {
        global.func
    } else {
        ctxt.state.blend_func
    };
    if force || ctxt.state.blend_func != func {
        unsafe {
            ctxt.glow.blend_func_separate(func.0, func.1, func.2, func.3);
        }
        ctxt.state.blend_func = func;
    }

    for (index, state) in &overrides {
        let index = *index as u32;
        unsafe {
            super::indexed::set_enabled(ctxt, glow::BLEND, index, state.enabled);
            if state.enabled {
                super::indexed::blend_equation(ctxt, index, state.equation);
                super::indexed::blend_func(ctxt, index, state.func);
            }
        }
    }
    ctxt.state.indexed_blend = !overrides.is_empty();

    Ok(())
}

/// Whether `equation` is one of the advanced blend equations of `AdvancedBlending`.
pub(crate) fn is_advanced(equation: gl::types::GLenum) -> bool {
    (0x9294..=0x92B0).contains(&equation)
}
