use crate::context;
use crate::gl;
use crate::ToGlEnum;
use glow::HasContext;

/// Describes the parameters that must be used for the stencil operations when drawing.
#[derive(Copy, Clone, Debug)]
pub struct Stencil {
    /// A comparison against the existing value in the stencil buffer.
    ///
    /// Only relevant for faces that are clockwise on the target surface. Other faces, points and
    /// lines use `stencil_test_counter_clockwise` instead.
    ///
    /// The default value is `AlwaysPass`.
    pub test_clockwise: StencilTest,

    /// Reference value that is used by `stencil_test_clockwise`, `stencil_fail_operation_clockwise`,
    /// `stencil_pass_depth_fail_operation_clockwise` and `stencil_depth_pass_operation_clockwise`.
    pub reference_value_clockwise: i32,

    /// Allows specifying a mask when writing data on the stencil buffer.
    ///
    /// Only relevant for faces that are clockwise on the target surface. Other faces, points and
    /// lines use `stencil_write_mask_counter_clockwise` instead.
    ///
    /// The default value is `0xffffffff`.
    pub write_mask_clockwise: u32,

    /// Specifies the operation to do when a fragment fails the stencil test.
    ///
    /// The stencil test is the test specified by `stencil_test_clockwise`.
    ///
    /// Only relevant for faces that are clockwise on the target surface. Other faces, points and
    /// lines use `stencil_fail_operation_counter_clockwise` instead.
    ///
    /// The default value is `Keep`.
    pub fail_operation_clockwise: StencilOperation,

    /// Specifies the operation to do when a fragment passes the stencil test but fails
    /// the depth test.
    ///
    /// The stencil test is the test specified by `stencil_test_clockwise`.
    ///
    /// Only relevant for faces that are clockwise on the target surface. Other faces, points and
    /// lines use `stencil_pass_depth_fail_operation_counter_clockwise` instead.
    ///
    /// The default value is `Keep`.
    pub pass_depth_fail_operation_clockwise: StencilOperation,

    /// Specifies the operation to do when a fragment passes both the stencil and depth tests.
    ///
    /// The stencil test is the test specified by `stencil_test_clockwise`.
    ///
    /// Only relevant for faces that are clockwise on the target surface. Other faces, points and
    /// lines use `stencil_depth_pass_operation_counter_clockwise` instead.
    ///
    /// The default value is `Keep`.
    pub depth_pass_operation_clockwise: StencilOperation,

    /// A comparaison against the existing value in the stencil buffer.
    ///
    /// Only relevant for points, lines and faces that are counter-clockwise on the target surface.
    /// Other faces use `stencil_test_counter_clockwise` instead.
    ///
    /// The default value is `AlwaysPass`.
    pub test_counter_clockwise: StencilTest,

    /// Reference value that is used by `stencil_test_counter_clockwise`,
    /// `stencil_fail_operation_counter_clockwise`,
    /// `stencil_pass_depth_fail_operation_counter_clockwise` and
    /// `stencil_depth_pass_operation_counter_clockwise`.
    pub reference_value_counter_clockwise: i32,

    /// Allows specifying a mask when writing data on the stencil buffer.
    ///
    /// Only relevant for points, lines and faces that are counter-clockwise on the target surface.
    /// Other faces use `stencil_write_mask_clockwise` instead.
    ///
    /// The default value is `0xffffffff`.
    pub write_mask_counter_clockwise: u32,

    /// Specifies the operation to do when a fragment fails the stencil test.
    ///
    /// The stencil test is the test specified by `stencil_test_counter_clockwise`.
    ///
    /// Only relevant for faces that are counter-clockwise on the target surface. Other faces
    /// use `stencil_fail_operation_clockwise` instead.
    ///
    /// The default value is `Keep`.
    pub fail_operation_counter_clockwise: StencilOperation,

    /// Specifies the operation to do when a fragment passes the stencil test but fails
    /// the depth test.
    ///
    /// The stencil test is the test specified by `stencil_test_counter_clockwise`.
    ///
    /// Only relevant for faces that are counter-clockwise on the target surface. Other faces
    /// use `stencil_pass_depth_fail_operation_clockwise` instead.
    ///
    /// The default value is `Keep`.
    pub pass_depth_fail_operation_counter_clockwise: StencilOperation,

    /// Specifies the operation to do when a fragment passes both the stencil and depth tests.
    ///
    /// The stencil test is the test specified by `stencil_test_counter_clockwise`.
    ///
    /// Only relevant for faces that are counter-clockwise on the target surface. Other faces
    /// use `stencil_depth_pass_operation_clockwise` instead.
    ///
    /// The default value is `Keep`.
    pub depth_pass_operation_counter_clockwise: StencilOperation,
}

impl Stencil {
    /// Returns true if these parameters read or write the stencil buffer. Otherwise the stencil
    /// test is disabled.
    pub fn requires_stencil_buffer(&self) -> bool {
        self.test_clockwise != StencilTest::AlwaysPass
            || self.test_counter_clockwise != StencilTest::AlwaysPass
            || self.fail_operation_clockwise != StencilOperation::Keep
            || self.pass_depth_fail_operation_clockwise != StencilOperation::Keep
            || self.depth_pass_operation_clockwise != StencilOperation::Keep
            || self.fail_operation_counter_clockwise != StencilOperation::Keep
            || self.pass_depth_fail_operation_counter_clockwise != StencilOperation::Keep
            || self.depth_pass_operation_counter_clockwise != StencilOperation::Keep
    }
}

impl Default for Stencil {
    #[inline]
    fn default() -> Stencil {
        Stencil {
            test_clockwise: StencilTest::AlwaysPass,
            reference_value_clockwise: 0,
            write_mask_clockwise: 0xffffffff,
            fail_operation_clockwise: StencilOperation::Keep,
            pass_depth_fail_operation_clockwise: StencilOperation::Keep,
            depth_pass_operation_clockwise: StencilOperation::Keep,
            test_counter_clockwise: StencilTest::AlwaysPass,
            reference_value_counter_clockwise: 0,
            write_mask_counter_clockwise: 0xffffffff,
            fail_operation_counter_clockwise: StencilOperation::Keep,
            pass_depth_fail_operation_counter_clockwise: StencilOperation::Keep,
            depth_pass_operation_counter_clockwise: StencilOperation::Keep,
        }
    }
}

/// Specifies which comparison the GPU will do to determine whether a sample passes the stencil
/// test. The general equation is `(ref & mask) CMP (stencil & mask)`, where `ref` is the reference
/// value (`stencil_reference_value_clockwise` or `stencil_reference_value_counter_clockwise`),
/// `CMP` is the comparison chosen, and `stencil` is the current value in the stencil buffer.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum StencilTest {
    /// The stencil test always passes.
    AlwaysPass,

    /// The stencil test always fails.
    AlwaysFail,

    /// `(ref & mask) < (stencil & mask)`
    IfLess {
        /// The mask that is and'ed with the reference value and stencil buffer.
        mask: u32,
    },

    /// `(ref & mask) <= (stencil & mask)`
    IfLessOrEqual {
        /// The mask that is and'ed with the reference value and stencil buffer.
        mask: u32,
    },

    /// `(ref & mask) > (stencil & mask)`
    IfMore {
        /// The mask that is and'ed with the reference value and stencil buffer.
        mask: u32,
    },

    /// `(ref & mask) >= (stencil & mask)`
    IfMoreOrEqual {
        /// The mask that is and'ed with the reference value and stencil buffer.
        mask: u32,
    },

    /// `(ref & mask) == (stencil & mask)`
    IfEqual {
        /// The mask that is and'ed with the reference value and stencil buffer.
        mask: u32,
    },

    /// `(ref & mask) != (stencil & mask)`
    IfNotEqual {
        /// The mask that is and'ed with the reference value and stencil buffer.
        mask: u32,
    },
}

/// Specificies which operation the GPU will do depending on the result of the stencil test.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(u32)] // GLenum
pub enum StencilOperation {
    /// Keeps the value currently in the stencil buffer.
    Keep = gl::KEEP,

    /// Writes zero in the stencil buffer.
    Zero = gl::ZERO,

    /// Writes the reference value (`stencil_reference_value_clockwise` or
    /// `stencil_reference_value_counter_clockwise`) in the stencil buffer.
    Replace = gl::REPLACE,

    /// Increments the value currently in the stencil buffer. If the value is the
    /// maximum, don't do anything.
    Increment = gl::INCR,

    /// Increments the value currently in the stencil buffer. If the value is the
    /// maximum, wrap to `0`.
    IncrementWrap = gl::INCR_WRAP,

    /// Decrements the value currently in the stencil buffer. If the value is `0`,
    /// don't do anything.
    Decrement = gl::DECR,

    /// Decrements the value currently in the stencil buffer. If the value is `0`,
    /// wrap to `-1`.
    DecrementWrap = gl::DECR_WRAP,

    /// Inverts each bit of the value.
    Invert = gl::INVERT,
}

impl ToGlEnum for StencilOperation {
    #[inline]
    fn to_glenum(&self) -> gl::types::GLenum {
        *self as gl::types::GLenum
    }
}

impl Stencil {
    /// Returns the settings with the clockwise and counter-clockwise ones exchanged.
    fn swap_windings(&self) -> Stencil {
        Stencil {
            test_clockwise: self.test_counter_clockwise,
            reference_value_clockwise: self.reference_value_counter_clockwise,
            write_mask_clockwise: self.write_mask_counter_clockwise,
            fail_operation_clockwise: self.fail_operation_counter_clockwise,
            pass_depth_fail_operation_clockwise: self.pass_depth_fail_operation_counter_clockwise,
            depth_pass_operation_clockwise: self.depth_pass_operation_counter_clockwise,
            test_counter_clockwise: self.test_clockwise,
            reference_value_counter_clockwise: self.reference_value_clockwise,
            write_mask_counter_clockwise: self.write_mask_clockwise,
            fail_operation_counter_clockwise: self.fail_operation_clockwise,
            pass_depth_fail_operation_counter_clockwise: self.pass_depth_fail_operation_clockwise,
            depth_pass_operation_counter_clockwise: self.depth_pass_operation_clockwise,
        }
    }
}

pub fn sync_stencil(
    ctxt: &mut context::CommandContext<'_>,
    params: &Stencil,
    front_face: super::FrontFace,
) {
    // the settings are named by the winding of the faces, while OpenGL names them front and
    // back: the code below sends the clockwise ones to the back faces
    let swapped;
    let params = if front_face == super::FrontFace::Clockwise {
        swapped = params.swap_windings();
        &swapped
    } else {
        params
    };

    // checks if stencil operations can be disabled
    if !params.requires_stencil_buffer() {
        if ctxt.state.enabled_stencil_test {
            unsafe { ctxt.glow.disable(glow::STENCIL_TEST) };
            ctxt.state.enabled_stencil_test = false;
        }

        return;
    }

    // we are now in "stencil enabled land"

    // enabling if necessary
    if !ctxt.state.enabled_stencil_test {
        unsafe { ctxt.glow.enable(glow::STENCIL_TEST) };
        ctxt.state.enabled_stencil_test = true;
    }

    // synchronizing the test and read masks
    let (test_cw, read_mask_cw) = match params.test_clockwise {
        StencilTest::AlwaysPass => (gl::ALWAYS, 0),
        StencilTest::AlwaysFail => (gl::NEVER, 0),
        StencilTest::IfLess { mask } => (gl::LESS, mask),
        StencilTest::IfLessOrEqual { mask } => (gl::LEQUAL, mask),
        StencilTest::IfMore { mask } => (gl::GREATER, mask),
        StencilTest::IfMoreOrEqual { mask } => (gl::GEQUAL, mask),
        StencilTest::IfEqual { mask } => (gl::EQUAL, mask),
        StencilTest::IfNotEqual { mask } => (gl::NOTEQUAL, mask),
    };

    let (test_ccw, read_mask_ccw) = match params.test_counter_clockwise {
        StencilTest::AlwaysPass => (gl::ALWAYS, 0),
        StencilTest::AlwaysFail => (gl::NEVER, 0),
        StencilTest::IfLess { mask } => (gl::LESS, mask),
        StencilTest::IfLessOrEqual { mask } => (gl::LEQUAL, mask),
        StencilTest::IfMore { mask } => (gl::GREATER, mask),
        StencilTest::IfMoreOrEqual { mask } => (gl::GEQUAL, mask),
        StencilTest::IfEqual { mask } => (gl::EQUAL, mask),
        StencilTest::IfNotEqual { mask } => (gl::NOTEQUAL, mask),
    };

    let ref_cw = params.reference_value_clockwise;
    let ref_ccw = params.reference_value_counter_clockwise;

    if (test_cw, ref_cw, read_mask_cw) == (test_ccw, ref_ccw, read_mask_ccw) {
        if ctxt.state.stencil_func_back != (test_cw, ref_cw, read_mask_cw)
            || ctxt.state.stencil_func_front != (test_ccw, ref_ccw, read_mask_ccw)
        {
            unsafe { ctxt.glow.stencil_func(test_cw, ref_cw, read_mask_cw) };
            ctxt.state.stencil_func_back = (test_cw, ref_cw, read_mask_cw);
            ctxt.state.stencil_func_front = (test_ccw, ref_ccw, read_mask_ccw);
        }
    } else {
        if ctxt.state.stencil_func_back != (test_cw, ref_cw, read_mask_cw) {
            unsafe {
                ctxt.glow
                    .stencil_func_separate(glow::BACK, test_cw, ref_cw, read_mask_cw)
            };
            ctxt.state.stencil_func_back = (test_cw, ref_cw, read_mask_cw);
        }

        if ctxt.state.stencil_func_front != (test_ccw, ref_ccw, read_mask_ccw) {
            unsafe {
                ctxt.glow
                    .stencil_func_separate(glow::FRONT, test_ccw, ref_ccw, read_mask_ccw)
            };
            ctxt.state.stencil_func_front = (test_ccw, ref_ccw, read_mask_ccw);
        }
    }

    // synchronizing the write mask
    if params.write_mask_clockwise == params.write_mask_counter_clockwise {
        if ctxt.state.stencil_mask_back != params.write_mask_clockwise
            || ctxt.state.stencil_mask_front != params.write_mask_clockwise
        {
            unsafe { ctxt.glow.stencil_mask(params.write_mask_clockwise) };
            ctxt.state.stencil_mask_back = params.write_mask_clockwise;
            ctxt.state.stencil_mask_front = params.write_mask_clockwise;
        }
    } else {
        if ctxt.state.stencil_mask_back != params.write_mask_clockwise {
            unsafe {
                ctxt.glow
                    .stencil_mask_separate(glow::BACK, params.write_mask_clockwise)
            };
            ctxt.state.stencil_mask_back = params.write_mask_clockwise;
        }

        if ctxt.state.stencil_mask_front != params.write_mask_counter_clockwise {
            unsafe {
                ctxt.glow
                    .stencil_mask_separate(glow::FRONT, params.write_mask_counter_clockwise)
            };
            ctxt.state.stencil_mask_front = params.write_mask_counter_clockwise;
        }
    }

    // synchronizing the operation
    let op_back = (
        params.fail_operation_clockwise.to_glenum(),
        params.pass_depth_fail_operation_clockwise.to_glenum(),
        params.depth_pass_operation_clockwise.to_glenum(),
    );

    let op_front = (
        params.fail_operation_counter_clockwise.to_glenum(),
        params
            .pass_depth_fail_operation_counter_clockwise
            .to_glenum(),
        params.depth_pass_operation_counter_clockwise.to_glenum(),
    );

    if op_back == op_front {
        if ctxt.state.stencil_op_back != op_back || ctxt.state.stencil_op_front != op_front {
            unsafe { ctxt.glow.stencil_op(op_back.0, op_back.1, op_back.2) };
            ctxt.state.stencil_op_back = op_back;
            ctxt.state.stencil_op_front = op_front;
        }
    } else {
        if ctxt.state.stencil_op_back != op_back {
            unsafe {
                ctxt.glow
                    .stencil_op_separate(glow::BACK, op_back.0, op_back.1, op_back.2)
            };
            ctxt.state.stencil_op_back = op_back;
        }

        if ctxt.state.stencil_op_front != op_front {
            unsafe {
                ctxt.glow
                    .stencil_op_separate(glow::FRONT, op_front.0, op_front.1, op_front.2)
            };
            ctxt.state.stencil_op_front = op_front;
        }
    }
}
