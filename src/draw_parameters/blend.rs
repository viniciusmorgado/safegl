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

    /// Multiply the source or destination component by its corresponding value
    /// in source index one (you need to explicitly specify `layout(location=0, index=1)`
    /// to bind it in your shader.
    /// This is useful in Dual Source Blending
    /// <https://www.khronos.org/registry/OpenGL/extensions/ARB/ARB_blend_func_extended.txt>
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
            }
        }

        #[inline(always)]
        fn blending_factors(
            blending_function: BlendingFunction,
        ) -> Option<(LinearBlendingFactor, LinearBlendingFactor)> {
            match blending_function {
                BlendingFunction::AlwaysReplace
                | BlendingFunction::Min
                | BlendingFunction::Max => None,
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
            ctxt.glow.blend_equation_separate(equation.0, equation.1);
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
            if state.enabled {
                ctxt.glow.enable_draw_buffer(glow::BLEND, index);
                ctxt.glow.blend_equation_separate_draw_buffer(
                    index,
                    state.equation.0,
                    state.equation.1,
                );
                ctxt.glow.blend_func_separate_draw_buffer(
                    index,
                    state.func.0,
                    state.func.1,
                    state.func.2,
                    state.func.3,
                );
            } else {
                ctxt.glow.disable_draw_buffer(glow::BLEND, index);
            }
        }
    }
    ctxt.state.indexed_blend = !overrides.is_empty();

    Ok(())
}
