use crate::RawUniformValue;
use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::BuildHasherDefault;

use fnv::FnvHasher;
use glow::HasContext;
use smallvec::SmallVec;

use crate::context::CommandContext;
use crate::gl;
use crate::handles::UniformLocations;
use crate::program::reflection::ShaderStage;

pub struct UniformsStorage {
    locations: UniformLocations,
    values:
        RefCell<HashMap<gl::types::GLint, Option<RawUniformValue>, BuildHasherDefault<FnvHasher>>>,
    uniform_blocks: RefCell<SmallVec<[Option<gl::types::GLuint>; 4]>>,
    shader_storage_blocks: RefCell<SmallVec<[Option<gl::types::GLuint>; 4]>>,
    subroutine_uniforms:
        RefCell<HashMap<ShaderStage, Vec<gl::types::GLuint>, BuildHasherDefault<FnvHasher>>>,
}

impl UniformsStorage {
    /// Builds a new empty storage.
    #[inline]
    pub fn new(locations: UniformLocations) -> UniformsStorage {
        UniformsStorage {
            locations,
            values: RefCell::new(HashMap::with_hasher(Default::default())),
            uniform_blocks: RefCell::new(SmallVec::new()),
            shader_storage_blocks: RefCell::new(SmallVec::new()),
            subroutine_uniforms: RefCell::new(HashMap::with_hasher(Default::default())),
        }
    }

    /// Compares `value` with the value stored in this object. If the values differ, updates
    /// the storage and calls `glUniform`.
    pub fn set_uniform_value(
        &self,
        ctxt: &mut CommandContext<'_>,
        program: glow::Program,
        location: gl::types::GLint,
        value: &RawUniformValue,
    ) {
        let mut values = self.values.borrow_mut();
        let locations = &self.locations;

        // TODO: don't assume that, instead use DSA if the program is not current
        assert!(ctxt.state.program == Some(program));

        macro_rules! uniform(
            ($ctxt:expr, $uniform:ident, $location:expr, $($params:expr),+) => (
                unsafe {
                    let location = locations.get($location);
                    $ctxt.glow.$uniform(Some(&location), $($params),+)
                }
            )
        );

        macro_rules! uniform_f64(
            ($ctxt:expr, $uniform:ident, $($params:expr),+) => (
                unsafe {
                    if $ctxt.extensions.gl_arb_gpu_shader_fp64 {
                        $ctxt.extra.$uniform($($params),+)
                    } else {
                        panic!("Double precision floats are not supported on this system.")
                    }
                }
            )
        );

        macro_rules! uniform_i64(
            ($ctxt:expr, $uniform:ident, $($params:expr),+) => (
                unsafe {
                    if $ctxt.extensions.gl_arb_gpu_shader_int64 {
                        $ctxt.extra.$uniform($($params),+)
                    } else {
                        panic!("64 bit integers are not supported on this system.")
                    }
                }
            )
        );

        match (value, values.entry(location).or_insert(None)) {
            (&RawUniformValue::SignedInt(a), &mut Some(RawUniformValue::SignedInt(b)))
                if a == b =>
            {
                ()
            }
            (&RawUniformValue::UnsignedInt(a), &mut Some(RawUniformValue::UnsignedInt(b)))
                if a == b =>
            {
                ()
            }
            (&RawUniformValue::Float(a), &mut Some(RawUniformValue::Float(b))) if a == b => (),
            (&RawUniformValue::Mat2(a), &mut Some(RawUniformValue::Mat2(b))) if a == b => (),
            (&RawUniformValue::Mat3(a), &mut Some(RawUniformValue::Mat3(b))) if a == b => (),
            (&RawUniformValue::Mat4(a), &mut Some(RawUniformValue::Mat4(b))) if a == b => (),
            (&RawUniformValue::Mat2x3(a), &mut Some(RawUniformValue::Mat2x3(b))) if a == b => (),
            (&RawUniformValue::Mat2x4(a), &mut Some(RawUniformValue::Mat2x4(b))) if a == b => (),
            (&RawUniformValue::Mat3x2(a), &mut Some(RawUniformValue::Mat3x2(b))) if a == b => (),
            (&RawUniformValue::Mat3x4(a), &mut Some(RawUniformValue::Mat3x4(b))) if a == b => (),
            (&RawUniformValue::Mat4x2(a), &mut Some(RawUniformValue::Mat4x2(b))) if a == b => (),
            (&RawUniformValue::Mat4x3(a), &mut Some(RawUniformValue::Mat4x3(b))) if a == b => (),
            (&RawUniformValue::Vec2(a), &mut Some(RawUniformValue::Vec2(b))) if a == b => (),
            (&RawUniformValue::Vec3(a), &mut Some(RawUniformValue::Vec3(b))) if a == b => (),
            (&RawUniformValue::Vec4(a), &mut Some(RawUniformValue::Vec4(b))) if a == b => (),
            (&RawUniformValue::IntVec2(a), &mut Some(RawUniformValue::IntVec2(b))) if a == b => (),
            (&RawUniformValue::IntVec3(a), &mut Some(RawUniformValue::IntVec3(b))) if a == b => (),
            (&RawUniformValue::IntVec4(a), &mut Some(RawUniformValue::IntVec4(b))) if a == b => (),
            (
                &RawUniformValue::UnsignedIntVec2(a),
                &mut Some(RawUniformValue::UnsignedIntVec2(b)),
            ) if a == b => (),
            (
                &RawUniformValue::UnsignedIntVec3(a),
                &mut Some(RawUniformValue::UnsignedIntVec3(b)),
            ) if a == b => (),
            (
                &RawUniformValue::UnsignedIntVec4(a),
                &mut Some(RawUniformValue::UnsignedIntVec4(b)),
            ) if a == b => (),
            (&RawUniformValue::Double(a), &mut Some(RawUniformValue::Double(b))) if a == b => (),
            (&RawUniformValue::DoubleMat2(a), &mut Some(RawUniformValue::DoubleMat2(b)))
                if a == b =>
            {
                ()
            }
            (&RawUniformValue::DoubleMat3(a), &mut Some(RawUniformValue::DoubleMat3(b)))
                if a == b =>
            {
                ()
            }
            (&RawUniformValue::DoubleMat4(a), &mut Some(RawUniformValue::DoubleMat4(b)))
                if a == b =>
            {
                ()
            }
            (&RawUniformValue::DoubleVec2(a), &mut Some(RawUniformValue::DoubleVec2(b)))
                if a == b =>
            {
                ()
            }
            (&RawUniformValue::DoubleVec3(a), &mut Some(RawUniformValue::DoubleVec3(b)))
                if a == b =>
            {
                ()
            }
            (&RawUniformValue::DoubleVec4(a), &mut Some(RawUniformValue::DoubleVec4(b)))
                if a == b =>
            {
                ()
            }
            (&RawUniformValue::Int64(a), &mut Some(RawUniformValue::Int64(b))) if a == b => (),
            (&RawUniformValue::Int64Vec2(a), &mut Some(RawUniformValue::Int64Vec2(b)))
                if a == b =>
            {
                ()
            }
            (&RawUniformValue::Int64Vec3(a), &mut Some(RawUniformValue::Int64Vec3(b)))
                if a == b =>
            {
                ()
            }
            (&RawUniformValue::Int64Vec4(a), &mut Some(RawUniformValue::Int64Vec4(b)))
                if a == b =>
            {
                ()
            }
            (&RawUniformValue::UnsignedInt64(a), &mut Some(RawUniformValue::UnsignedInt64(b)))
                if a == b =>
            {
                ()
            }
            (
                &RawUniformValue::UnsignedInt64Vec2(a),
                &mut Some(RawUniformValue::UnsignedInt64Vec2(b)),
            ) if a == b => (),
            (
                &RawUniformValue::UnsignedInt64Vec3(a),
                &mut Some(RawUniformValue::UnsignedInt64Vec3(b)),
            ) if a == b => (),
            (
                &RawUniformValue::UnsignedInt64Vec4(a),
                &mut Some(RawUniformValue::UnsignedInt64Vec4(b)),
            ) if a == b => (),

            (&RawUniformValue::SignedInt(v), target) => {
                *target = Some(RawUniformValue::SignedInt(v));
                uniform!(ctxt, uniform_1_i32, location, v);
            }

            (&RawUniformValue::UnsignedInt(v), target) => {
                *target = Some(RawUniformValue::UnsignedInt(v));
                uniform!(ctxt, uniform_1_u32, location, v);
            }

            (&RawUniformValue::Float(v), target) => {
                *target = Some(RawUniformValue::Float(v));
                uniform!(ctxt, uniform_1_f32, location, v);
            }

            (&RawUniformValue::Mat2(v), target) => {
                *target = Some(RawUniformValue::Mat2(v));
                uniform!(ctxt, uniform_matrix_2_f32_slice, location, false, v.as_flattened());
            }

            (&RawUniformValue::Mat3(v), target) => {
                *target = Some(RawUniformValue::Mat3(v));
                uniform!(ctxt, uniform_matrix_3_f32_slice, location, false, v.as_flattened());
            }

            (&RawUniformValue::Mat4(v), target) => {
                *target = Some(RawUniformValue::Mat4(v));
                uniform!(ctxt, uniform_matrix_4_f32_slice, location, false, v.as_flattened());
            }

            (&RawUniformValue::Mat2x3(v), target) => {
                *target = Some(RawUniformValue::Mat2x3(v));
                uniform!(ctxt, uniform_matrix_2x3_f32_slice, location, false, v.as_flattened());
            }

            (&RawUniformValue::Mat2x4(v), target) => {
                *target = Some(RawUniformValue::Mat2x4(v));
                uniform!(ctxt, uniform_matrix_2x4_f32_slice, location, false, v.as_flattened());
            }

            (&RawUniformValue::Mat3x2(v), target) => {
                *target = Some(RawUniformValue::Mat3x2(v));
                uniform!(ctxt, uniform_matrix_3x2_f32_slice, location, false, v.as_flattened());
            }

            (&RawUniformValue::Mat3x4(v), target) => {
                *target = Some(RawUniformValue::Mat3x4(v));
                uniform!(ctxt, uniform_matrix_3x4_f32_slice, location, false, v.as_flattened());
            }

            (&RawUniformValue::Mat4x2(v), target) => {
                *target = Some(RawUniformValue::Mat4x2(v));
                uniform!(ctxt, uniform_matrix_4x2_f32_slice, location, false, v.as_flattened());
            }

            (&RawUniformValue::Mat4x3(v), target) => {
                *target = Some(RawUniformValue::Mat4x3(v));
                uniform!(ctxt, uniform_matrix_4x3_f32_slice, location, false, v.as_flattened());
            }

            (&RawUniformValue::Vec2(v), target) => {
                *target = Some(RawUniformValue::Vec2(v));
                uniform!(ctxt, uniform_2_f32_slice, location, &v);
            }

            (&RawUniformValue::Vec3(v), target) => {
                *target = Some(RawUniformValue::Vec3(v));
                uniform!(ctxt, uniform_3_f32_slice, location, &v);
            }

            (&RawUniformValue::Vec4(v), target) => {
                *target = Some(RawUniformValue::Vec4(v));
                uniform!(ctxt, uniform_4_f32_slice, location, &v);
            }

            (&RawUniformValue::IntVec2(v), target) => {
                *target = Some(RawUniformValue::IntVec2(v));
                uniform!(ctxt, uniform_2_i32_slice, location, &v);
            }

            (&RawUniformValue::IntVec3(v), target) => {
                *target = Some(RawUniformValue::IntVec3(v));
                uniform!(ctxt, uniform_3_i32_slice, location, &v);
            }

            (&RawUniformValue::IntVec4(v), target) => {
                *target = Some(RawUniformValue::IntVec4(v));
                uniform!(ctxt, uniform_4_i32_slice, location, &v);
            }

            (&RawUniformValue::UnsignedIntVec2(v), target) => {
                *target = Some(RawUniformValue::UnsignedIntVec2(v));
                uniform!(ctxt, uniform_2_u32_slice, location, &v);
            }

            (&RawUniformValue::UnsignedIntVec3(v), target) => {
                *target = Some(RawUniformValue::UnsignedIntVec3(v));
                uniform!(ctxt, uniform_3_u32_slice, location, &v);
            }

            (&RawUniformValue::UnsignedIntVec4(v), target) => {
                *target = Some(RawUniformValue::UnsignedIntVec4(v));
                uniform!(ctxt, uniform_4_u32_slice, location, &v);
            }
            (&RawUniformValue::Double(v), target) => {
                *target = Some(RawUniformValue::Double(v));
                uniform_f64!(ctxt, Uniform1d, location, v);
            }

            (&RawUniformValue::DoubleMat2(v), target) => {
                *target = Some(RawUniformValue::DoubleMat2(v));
                uniform_f64!(
                    ctxt,
                    UniformMatrix2dv,
                    location,
                    1,
                    gl::FALSE,
                    v.as_ptr() as *const gl::types::GLdouble
                );
            }

            (&RawUniformValue::DoubleMat3(v), target) => {
                *target = Some(RawUniformValue::DoubleMat3(v));
                uniform_f64!(
                    ctxt,
                    UniformMatrix3dv,
                    location,
                    1,
                    gl::FALSE,
                    v.as_ptr() as *const gl::types::GLdouble
                );
            }

            (&RawUniformValue::DoubleMat4(v), target) => {
                *target = Some(RawUniformValue::DoubleMat4(v));
                uniform_f64!(
                    ctxt,
                    UniformMatrix4dv,
                    location,
                    1,
                    gl::FALSE,
                    v.as_ptr() as *const gl::types::GLdouble
                );
            }

            (&RawUniformValue::DoubleVec2(v), target) => {
                *target = Some(RawUniformValue::DoubleVec2(v));
                uniform_f64!(
                    ctxt,
                    Uniform2dv,
                    location,
                    1,
                    v.as_ptr() as *const gl::types::GLdouble
                );
            }

            (&RawUniformValue::DoubleVec3(v), target) => {
                *target = Some(RawUniformValue::DoubleVec3(v));
                uniform_f64!(
                    ctxt,
                    Uniform3dv,
                    location,
                    1,
                    v.as_ptr() as *const gl::types::GLdouble
                );
            }

            (&RawUniformValue::DoubleVec4(v), target) => {
                *target = Some(RawUniformValue::DoubleVec4(v));
                uniform_f64!(
                    ctxt,
                    Uniform4dv,
                    location,
                    1,
                    v.as_ptr() as *const gl::types::GLdouble
                );
            }
            (&RawUniformValue::Int64(v), target) => {
                *target = Some(RawUniformValue::Int64(v));
                uniform_i64!(ctxt, Uniform1i64ARB, location, v);
            }
            (&RawUniformValue::Int64Vec2(v), target) => {
                *target = Some(RawUniformValue::Int64Vec2(v));
                uniform_i64!(
                    ctxt,
                    Uniform2i64vARB,
                    location,
                    1,
                    v.as_ptr() as *const gl::types::GLint64
                );
            }

            (&RawUniformValue::Int64Vec3(v), target) => {
                *target = Some(RawUniformValue::Int64Vec3(v));
                uniform_i64!(
                    ctxt,
                    Uniform3i64vARB,
                    location,
                    1,
                    v.as_ptr() as *const gl::types::GLint64
                );
            }

            (&RawUniformValue::Int64Vec4(v), target) => {
                *target = Some(RawUniformValue::Int64Vec4(v));
                uniform_i64!(
                    ctxt,
                    Uniform4i64vARB,
                    location,
                    1,
                    v.as_ptr() as *const gl::types::GLint64
                );
            }
            (&RawUniformValue::UnsignedInt64(v), target) => {
                *target = Some(RawUniformValue::UnsignedInt64(v));
                uniform_i64!(ctxt, Uniform1ui64ARB, location, v);
            }
            (&RawUniformValue::UnsignedInt64Vec2(v), target) => {
                *target = Some(RawUniformValue::UnsignedInt64Vec2(v));
                uniform_i64!(
                    ctxt,
                    Uniform2ui64vARB,
                    location,
                    1,
                    v.as_ptr() as *const gl::types::GLuint64
                );
            }

            (&RawUniformValue::UnsignedInt64Vec3(v), target) => {
                *target = Some(RawUniformValue::UnsignedInt64Vec3(v));
                uniform_i64!(
                    ctxt,
                    Uniform3ui64vARB,
                    location,
                    1,
                    v.as_ptr() as *const gl::types::GLuint64
                );
            }

            (&RawUniformValue::UnsignedInt64Vec4(v), target) => {
                *target = Some(RawUniformValue::UnsignedInt64Vec4(v));
                uniform_i64!(
                    ctxt,
                    Uniform4ui64vARB,
                    location,
                    1,
                    v.as_ptr() as *const gl::types::GLuint64
                );
            }
        }
    }

    /// Compares `value` with the value stored in this object. If the values differ, updates
    /// the storage and calls `glUniformBlockBinding`.
    pub fn set_uniform_block_binding(
        &self,
        ctxt: &mut CommandContext<'_>,
        program: glow::Program,
        location: gl::types::GLuint,
        value: gl::types::GLuint,
    ) {
        let mut blocks = self.uniform_blocks.borrow_mut();

        if blocks.len() <= location as usize {
            for _ in blocks.len()..location as usize + 1 {
                blocks.push(None);
            }
        }

        // TODO: don't assume that, instead use DSA if the program is not current
        assert!(ctxt.state.program == Some(program));

        match (value, &mut blocks[location as usize]) {
            (a, &mut Some(b)) if a == b => (),

            (a, target) => {
                *target = Some(a);
                unsafe {
                    ctxt.glow.uniform_block_binding(program, location, value);
                }
            }
        }
    }

    /// Compares `value` with the value stored in this object. If the values differ, updates
    /// the storage and calls `glShaderStorageBlockBinding`.
    pub fn set_shader_storage_block_binding(
        &self,
        ctxt: &mut CommandContext<'_>,
        program: glow::Program,
        location: gl::types::GLuint,
        value: gl::types::GLuint,
    ) {
        let mut blocks = self.shader_storage_blocks.borrow_mut();

        if blocks.len() <= location as usize {
            for _ in blocks.len()..location as usize + 1 {
                blocks.push(None);
            }
        }

        // TODO: don't assume that, instead use DSA if the program is not current
        assert!(ctxt.state.program == Some(program));

        match (value, &mut blocks[location as usize]) {
            (a, &mut Some(b)) if a == b => (),

            (a, target) => {
                *target = Some(a);
                unsafe {
                    ctxt.glow
                        .shader_storage_block_binding(program, location, value);
                }
            }
        }
    }

    /// Clears all subroutine uniform values stored in this object.
    /// This needs to be called when changing programs without `use_program`,
    /// since all subroutine uniform state is lost when changing programs.
    #[inline]
    pub(crate) fn flush_subroutine_uniforms(&self) {
        let mut subroutine_uniforms = self.subroutine_uniforms.borrow_mut();
        if !subroutine_uniforms.is_empty() {
            subroutine_uniforms.clear();
        }
    }

    /// Compares `indices` to the value stored in this object. If the values differ,
    /// updates the programs subroutine uniform bindings.
    pub fn set_subroutine_uniforms_for_stage(
        &self,
        ctxt: &mut CommandContext<'_>,
        program: glow::Program,
        stage: ShaderStage,
        indices: &[gl::types::GLuint],
    ) {
        let mut subroutine_uniforms = self.subroutine_uniforms.borrow_mut();
        if let Some(stored_indices) = subroutine_uniforms.get(&stage) {
            if &stored_indices[..] == indices {
                return;
            }
        }
        // TODO: don't assume that, instead use DSA if the program is not current
        assert!(ctxt.state.program == Some(program));
        subroutine_uniforms.insert(stage, indices.to_vec());
        unsafe {
            ctxt.extra.UniformSubroutinesuiv(
                stage.to_gl_enum(),
                indices.len() as gl::types::GLsizei,
                indices.as_ptr() as *const _,
            );
        }
    }
}
