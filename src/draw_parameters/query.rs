use crate::backend::Facade;
use crate::context::CommandContext;
use crate::context::Context;
use crate::ContextExt;
use crate::DrawError;
use crate::GlObject;
use crate::QueryExt;
use crate::ToGlEnum;
use glow::HasContext;

use std::cell::Cell;
use std::error::Error;
use std::fmt;
use std::rc::Rc;

use smallvec::SmallVec;

#[cfg(safegl_gl44)]
use crate::buffer::Buffer;
#[cfg(safegl_gl44)]
use crate::buffer::BufferSlice;
#[cfg(safegl_gl44)]
use crate::BufferExt;
#[cfg(safegl_gl44)]
use crate::BufferSliceExt;

use crate::gl;
use crate::handles::RawName;
use crate::version::Api;
use crate::version::Version;

pub struct RawQuery {
    context: Rc<Context>,
    id: glow::Query,
    ty: QueryType,

    // true means that this query has already been used or is being used to get data
    // this is important to know because we want to avoid erasing data
    has_been_used: Cell<bool>,

    /// The label given before the query began, when OpenGL creates the query object.
    #[cfg(any(safegl_gl43, safegl_gles32))]
    pending_label: std::cell::RefCell<Option<String>>,
}

#[derive(Copy, Clone)]
pub enum QueryType {
    SamplesPassed,
    AnySamplesPassed,
    AnySamplesPassedConservative,
    TimeElapsed,
    Timestamp,
    /// The primitives generated for a vertex stream.
    PrimitivesGenerated(u32),
    /// The primitives written by transform feedback for a vertex stream.
    TransformFeedbackPrimitivesWritten(u32),
    #[cfg(safegl_gl46)]
    PipelineStatistic(PipelineStatistic),
    #[cfg(safegl_gl46)]
    TransformFeedbackOverflow,
    #[cfg(safegl_gl46)]
    TransformFeedbackStreamOverflow(u32),
}

impl QueryType {
    /// The target and index of the queries that `indexed_queries` tracks.
    fn indexed_target(&self) -> Option<(gl::types::GLenum, gl::types::GLuint)> {
        match *self {
            QueryType::PrimitivesGenerated(stream) => Some((gl::PRIMITIVES_GENERATED, stream)),
            QueryType::TransformFeedbackPrimitivesWritten(stream) => {
                Some((gl::TRANSFORM_FEEDBACK_PRIMITIVES_WRITTEN, stream))
            }
            #[cfg(safegl_gl46)]
            QueryType::PipelineStatistic(statistic) => Some((statistic.to_glenum(), 0)),
            #[cfg(safegl_gl46)]
            QueryType::TransformFeedbackOverflow => Some((gl::TRANSFORM_FEEDBACK_OVERFLOW, 0)),
            #[cfg(safegl_gl46)]
            QueryType::TransformFeedbackStreamOverflow(stream) => {
                Some((gl::TRANSFORM_FEEDBACK_STREAM_OVERFLOW, stream))
            }
            _ => None,
        }
    }
}

/// A statistic of the pipeline, which a `PipelineStatisticsQuery` counts.
#[cfg(safegl_gl46)]
#[cfg_attr(docsrs, doc(cfg(safegl_gl46)))]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum PipelineStatistic {
    /// The vertices that the draws submit (`GL_VERTICES_SUBMITTED`).
    VerticesSubmitted,
    /// The primitives that the draws submit (`GL_PRIMITIVES_SUBMITTED`).
    PrimitivesSubmitted,
    /// The invocations of the vertex shader (`GL_VERTEX_SHADER_INVOCATIONS`), which can be fewer
    /// than the vertices when the GPU reuses the results of repeated indices.
    VertexShaderInvocations,
    /// The patches that the tessellation control shader processes
    /// (`GL_TESS_CONTROL_SHADER_PATCHES`).
    TessellationControlShaderPatches,
    /// The invocations of the tessellation evaluation shader
    /// (`GL_TESS_EVALUATION_SHADER_INVOCATIONS`).
    TessellationEvaluationShaderInvocations,
    /// The invocations of the geometry shader (`GL_GEOMETRY_SHADER_INVOCATIONS`).
    GeometryShaderInvocations,
    /// The primitives that the geometry shader emits (`GL_GEOMETRY_SHADER_PRIMITIVES_EMITTED`).
    GeometryShaderPrimitivesEmitted,
    /// The invocations of the fragment shader (`GL_FRAGMENT_SHADER_INVOCATIONS`).
    FragmentShaderInvocations,
    /// The invocations of compute shaders (`GL_COMPUTE_SHADER_INVOCATIONS`), counted by
    /// `ComputeShader::execute_with_statistics`.
    ComputeShaderInvocations,
    /// The primitives that reach the clipping stage (`GL_CLIPPING_INPUT_PRIMITIVES`).
    ClippingInputPrimitives,
    /// The primitives that leave the clipping stage (`GL_CLIPPING_OUTPUT_PRIMITIVES`).
    ClippingOutputPrimitives,
}

#[cfg(safegl_gl46)]
impl ToGlEnum for PipelineStatistic {
    #[inline]
    fn to_glenum(&self) -> gl::types::GLenum {
        match *self {
            PipelineStatistic::VerticesSubmitted => gl::VERTICES_SUBMITTED,
            PipelineStatistic::PrimitivesSubmitted => gl::PRIMITIVES_SUBMITTED,
            PipelineStatistic::VertexShaderInvocations => gl::VERTEX_SHADER_INVOCATIONS,
            PipelineStatistic::TessellationControlShaderPatches => gl::TESS_CONTROL_SHADER_PATCHES,
            PipelineStatistic::TessellationEvaluationShaderInvocations => {
                gl::TESS_EVALUATION_SHADER_INVOCATIONS
            }
            PipelineStatistic::GeometryShaderInvocations => gl::GEOMETRY_SHADER_INVOCATIONS,
            PipelineStatistic::GeometryShaderPrimitivesEmitted => {
                gl::GEOMETRY_SHADER_PRIMITIVES_EMITTED
            }
            PipelineStatistic::FragmentShaderInvocations => gl::FRAGMENT_SHADER_INVOCATIONS,
            PipelineStatistic::ComputeShaderInvocations => gl::COMPUTE_SHADER_INVOCATIONS,
            PipelineStatistic::ClippingInputPrimitives => gl::CLIPPING_INPUT_PRIMITIVES,
            PipelineStatistic::ClippingOutputPrimitives => gl::CLIPPING_OUTPUT_PRIMITIVES,
        }
    }
}

impl ToGlEnum for QueryType {
    #[inline]
    fn to_glenum(&self) -> gl::types::GLenum {
        match *self {
            QueryType::SamplesPassed => gl::SAMPLES_PASSED,
            QueryType::AnySamplesPassed => gl::ANY_SAMPLES_PASSED,
            QueryType::AnySamplesPassedConservative => gl::ANY_SAMPLES_PASSED_CONSERVATIVE,
            QueryType::TimeElapsed => gl::TIME_ELAPSED,
            QueryType::Timestamp => gl::TIMESTAMP,
            QueryType::PrimitivesGenerated(_) => gl::PRIMITIVES_GENERATED,
            QueryType::TransformFeedbackPrimitivesWritten(_) => {
                gl::TRANSFORM_FEEDBACK_PRIMITIVES_WRITTEN
            }
            #[cfg(safegl_gl46)]
            QueryType::PipelineStatistic(statistic) => statistic.to_glenum(),
            #[cfg(safegl_gl46)]
            QueryType::TransformFeedbackOverflow => gl::TRANSFORM_FEEDBACK_OVERFLOW,
            #[cfg(safegl_gl46)]
            QueryType::TransformFeedbackStreamOverflow(_) => gl::TRANSFORM_FEEDBACK_STREAM_OVERFLOW,
        }
    }
}

/// Error that can happen when creating a query object.
///
/// New variants can be added without a major version, so matches need a wildcard arm.
#[derive(Copy, Clone, Debug)]
#[non_exhaustive]
pub enum QueryCreationError {
    /// The given query type is not supported.
    NotSupported,

    /// The context is lost: nothing can be created in it.
    ContextLost,
}

impl fmt::Display for QueryCreationError {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        use self::QueryCreationError::*;
        let desc = match *self {
            NotSupported => "The given query type is not supported",
            ContextLost => "The context is lost",
        };
        fmt.write_str(desc)
    }
}

impl Error for QueryCreationError {}

/// Error that can happen when writing the value of a query to a buffer.
#[cfg(safegl_gl44)]
#[cfg_attr(docsrs, doc(cfg(safegl_gl44)))]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ToBufferError {
    /// Writing the result to a buffer is not supported: it needs OpenGL 4.4,
    /// `GL_ARB_query_buffer_object` or `GL_AMD_query_buffer_object`.
    NotSupported,

    /// The buffer belongs to another context than the query.
    ContextMismatch,
}

#[cfg(safegl_gl44)]
impl fmt::Display for ToBufferError {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        use self::ToBufferError::*;
        let desc = match *self {
            NotSupported => "Writing the result to a buffer is not supported",
            ContextMismatch => "The buffer belongs to another context than the query",
        };
        fmt.write_str(desc)
    }
}

#[cfg(safegl_gl44)]
impl Error for ToBufferError {}

/// What a query writes to a buffer.
#[cfg(safegl_gl44)]
#[cfg_attr(docsrs, doc(cfg(safegl_gl44)))]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum QueryBufferMode {
    /// The result, which the GPU waits for before running the next commands
    /// (`GL_QUERY_RESULT`).
    Result,

    /// The result if it is available when the GPU runs the command, and nothing otherwise: the
    /// buffer keeps its value (`GL_QUERY_RESULT_NO_WAIT`).
    ResultNoWait,

    /// 1 if the result is available when the GPU runs the command, and 0 otherwise
    /// (`GL_QUERY_RESULT_AVAILABLE`).
    Availability,
}

#[cfg(safegl_gl44)]
impl QueryBufferMode {
    fn to_glenum(self) -> gl::types::GLenum {
        match self {
            QueryBufferMode::Result => gl::QUERY_RESULT,
            QueryBufferMode::ResultNoWait => gl::QUERY_RESULT_NO_WAIT,
            QueryBufferMode::Availability => gl::QUERY_RESULT_AVAILABLE,
        }
    }
}

#[cfg(safegl_gl44)]
mod query_value {
    use crate::gl;

    pub trait Sealed: Sized {
        /// The value of a query that was never used: its result is 0 and available.
        fn unused(mode: super::QueryBufferMode) -> Self;

        /// Calls the `glGetQueryObject*v` of the type, which writes at `offset` in the query
        /// buffer.
        unsafe fn write(
            extra: &gl::Extra,
            query: gl::types::GLuint,
            pname: gl::types::GLenum,
            offset: usize,
        );
    }

    macro_rules! value {
        ($ty:ty, $get:ident) => {
            impl Sealed for $ty {
                fn unused(mode: super::QueryBufferMode) -> $ty {
                    (mode == super::QueryBufferMode::Availability) as $ty
                }

                unsafe fn write(
                    extra: &gl::Extra,
                    query: gl::types::GLuint,
                    pname: gl::types::GLenum,
                    offset: usize,
                ) {
                    extra.$get(query, pname, offset as *mut _)
                }
            }
        };
    }

    value!(u32, GetQueryObjectuiv);
    value!(i32, GetQueryObjectiv);
    value!(u64, GetQueryObjectui64v);
    value!(i64, GetQueryObjecti64v);
}

/// A type that queries can write to buffers: `u32`, `i32`, `u64` or `i64`. A result that doesn't
/// fit the type is truncated.
#[cfg(safegl_gl44)]
#[cfg_attr(docsrs, doc(cfg(safegl_gl44)))]
pub trait QueryBufferValue: query_value::Sealed + crate::buffer::Content + Copy + 'static {}

#[cfg(safegl_gl44)]
impl QueryBufferValue for u32 {}
#[cfg(safegl_gl44)]
impl QueryBufferValue for i32 {}
#[cfg(safegl_gl44)]
impl QueryBufferValue for u64 {}
#[cfg(safegl_gl44)]
impl QueryBufferValue for i64 {}

/// Writes the result of the query `query` to `target`, or its availability. `used` is false for
/// a query that never began, which OpenGL refuses to write.
#[cfg(safegl_gl44)]
pub(crate) fn write_query_to_buffer<T: QueryBufferValue>(
    context: &Context,
    query: glow::Query,
    used: bool,
    target: BufferSlice<'_, T>,
    mode: QueryBufferMode,
) -> Result<(), ToBufferError> {
    let mut ctxt = context.make_current();

    if !(ctxt.version >= &Version(Api::Gl, 4, 4)
        || ctxt.extensions.gl_arb_query_buffer_object
        || ctxt.extensions.gl_amd_query_buffer_object)
    {
        return Err(ToBufferError::NotSupported);
    }
    if !ctxt.owns(target.get_context()) {
        return Err(ToBufferError::ContextMismatch);
    }

    if !used {
        // the result of a query that was never used is 0 and available, like `get` returns,
        // but OpenGL refuses to write the result of a query object that never started
        drop(ctxt);
        target.write(&T::unused(mode));
        return Ok(());
    }

    assert!(target
        .get_offset_bytes()
        .is_multiple_of(std::mem::size_of::<T>()));

    target.prepare_and_bind_for_query(&mut ctxt);
    unsafe {
        T::write(
            ctxt.extra,
            query.raw_name(),
            mode.to_glenum(),
            target.get_offset_bytes(),
        );
    }

    if let Some(fence) = target.add_fence() {
        fence.insert(&mut ctxt);
    }

    Ok(())
}

/// Error that can happen when getting the result of a query.
///
/// New variants can be added without a major version, so matches need a wildcard arm.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum QueryResultError {
    /// The OpenGL context has been lost, so the result of the query is lost too.
    ContextLost,

    /// The result isn't available yet, and WebGL can't wait for it: it makes the results of
    /// queries available between two tasks of the browser's event loop, such as the next
    /// animation frame, once `is_ready` returns true.
    NotAvailable,
}

impl fmt::Display for QueryResultError {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        let desc = match *self {
            QueryResultError::ContextLost => {
                "The OpenGL context has been lost, so the result of the query is lost too"
            }
            QueryResultError::NotAvailable => {
                "WebGL makes the result available in a later task of the browser's event loop"
            }
        };
        fmt.write_str(desc)
    }
}

impl Error for QueryResultError {}

impl RawQuery {
    /// Builds a new query. Returns `None` if the backend doesn't support this type.
    pub fn new<F: ?Sized>(facade: &F, ty: QueryType) -> Result<RawQuery, QueryCreationError>
    where
        F: Facade,
    {
        let context = facade.get_context().clone();
        let ctxt = facade.get_context().make_current();
        // the objects of a lost context are gone, whether the browser creates them or not
        if ctxt.objects_lost() {
            return Err(QueryCreationError::ContextLost);
        }

        // FIXME: handle Timestamp separately

        let id = unsafe {
            if ctxt.version.0 == Api::Gl {
                match ty {
                    QueryType::AnySamplesPassed
                    | QueryType::SamplesPassed
                    | QueryType::TimeElapsed => (),
                    QueryType::PrimitivesGenerated(stream)
                    | QueryType::TransformFeedbackPrimitivesWritten(stream)
                        if (stream as gl::types::GLint)
                            < ctxt.glow.get_parameter_i32(glow::MAX_VERTEX_STREAMS) => {}
                    QueryType::AnySamplesPassedConservative
                        if ctxt.extensions.gl_arb_es3_compatibility
                            || ctxt.version >= &Version(Api::Gl, 4, 3) =>
                    {
                        ()
                    }
                    #[cfg(safegl_gl46)]
                    QueryType::PipelineStatistic(_)
                        if ctxt.version >= &Version(Api::Gl, 4, 6)
                            || ctxt.extensions.gl_arb_pipeline_statistics_query => {}
                    #[cfg(safegl_gl46)]
                    QueryType::TransformFeedbackOverflow
                        if ctxt.version >= &Version(Api::Gl, 4, 6)
                            || ctxt.extensions.gl_arb_transform_feedback_overflow_query => {}
                    #[cfg(safegl_gl46)]
                    QueryType::TransformFeedbackStreamOverflow(stream)
                        if (ctxt.version >= &Version(Api::Gl, 4, 6)
                            || ctxt.extensions.gl_arb_transform_feedback_overflow_query)
                            && (stream as gl::types::GLint)
                                < ctxt.glow.get_parameter_i32(glow::MAX_VERTEX_STREAMS) => {}
                    _ => return Err(QueryCreationError::NotSupported),
                };

                ctxt.glow
                    .create_query()
                    .map_err(|_| QueryCreationError::ContextLost)?
            } else {
                // OpenGL ES 3.0, plus the queries of geometry shaders and of
                // `GL_EXT_disjoint_timer_query`
                match ty {
                    QueryType::AnySamplesPassed
                    | QueryType::AnySamplesPassedConservative
                    | QueryType::TransformFeedbackPrimitivesWritten(0) => (),
                    QueryType::PrimitivesGenerated(0)
                        if ctxt.version >= &Version(Api::GlEs, 3, 2)
                            || ctxt.extensions.gl_ext_geometry_shader
                            || ctxt.extensions.gl_oes_geometry_shader => {}
                    QueryType::TimeElapsed if ctxt.extensions.gl_ext_disjoint_timer_query => {}
                    _ => return Err(QueryCreationError::NotSupported),
                };

                ctxt.glow
                    .create_query()
                    .map_err(|_| QueryCreationError::ContextLost)?
            }
        };

        Ok(RawQuery {
            context,
            id,
            ty,
            has_been_used: Cell::new(false),
            #[cfg(any(safegl_gl43, safegl_gles32))]
            pending_label: Default::default(),
        })
    }

    /// Records that the query has begun, which creates its OpenGL object, and gives the object
    /// the label it was given before.
    fn mark_used(&self, ctxt: &mut CommandContext<'_>) {
        self.has_been_used.set(true);
        #[cfg(any(safegl_gl43, safegl_gles32))]
        if let Some(label) = self.pending_label.borrow_mut().take() {
            unsafe {
                ctxt.glow
                    .object_label(gl::QUERY, self.id.0.get(), Some(label))
            };
        }
        #[cfg(not(any(safegl_gl43, safegl_gles32)))]
        let _ = ctxt;
    }

    /// Queries the counter to see if the result is already available.
    ///
    /// Returns `false` if the context has been lost: the result will never be available.
    pub fn is_ready(&self) -> bool {
        if self.context.is_context_lost() {
            return false;
        }

        let mut ctxt = self.context.make_current();
        self.deactivate(&mut ctxt);

        // the result of a query that never began is 0, which `get` returns at once
        if !self.has_been_used.get() {
            return true;
        }

        #[cfg(safegl_gl44)]
        Buffer::<u8>::unbind_query(&mut ctxt);

        unsafe {
            let value = ctxt
                .glow
                .get_query_parameter_u32(self.id, glow::QUERY_RESULT_AVAILABLE);

            value != 0
        }
    }

    /// Returns the value of the query. Blocks until it is available.
    ///
    /// This function doesn't block if `is_ready` returns true.
    pub fn get_u32(&self) -> Result<u32, QueryResultError> {
        if self.context.is_context_lost() {
            return Err(QueryResultError::ContextLost);
        }

        let mut ctxt = self.context.make_current();
        self.deactivate(&mut ctxt);

        if !self.has_been_used.get() {
            return Ok(0);
        }

        #[cfg(safegl_gl44)]
        Buffer::<u8>::unbind_query(&mut ctxt);
        self.check_available(&mut ctxt)?;

        unsafe {
            Ok(ctxt
                .glow
                .get_query_parameter_u32(self.id, glow::QUERY_RESULT))
        }
    }

    /// Writes the value of the query to a buffer.
    #[cfg(safegl_gl44)]
    pub fn write_to_buffer<T: QueryBufferValue>(
        &self,
        target: BufferSlice<'_, T>,
        mode: QueryBufferMode,
    ) -> Result<(), ToBufferError> {
        {
            let mut ctxt = self.context.make_current();
            self.deactivate(&mut ctxt);
        }
        write_query_to_buffer(
            &self.context,
            self.id,
            self.has_been_used.get(),
            target,
            mode,
        )
    }

    /// Returns the value of the query. Blocks until it is available.
    ///
    /// This function doesn't block if `is_ready` returns true.
    pub fn get_u64(&self) -> Result<u64, QueryResultError> {
        if self.context.is_context_lost() {
            return Err(QueryResultError::ContextLost);
        }

        let mut ctxt = self.context.make_current();
        self.deactivate(&mut ctxt);

        if !self.has_been_used.get() {
            return Ok(0);
        }

        #[cfg(safegl_gl44)]
        Buffer::<u8>::unbind_query(&mut ctxt);
        self.check_available(&mut ctxt)?;

        unsafe {
            if let Ok(value) = self.raw_get_u64(&mut ctxt) {
                return Ok(value);
            }

            let value = ctxt
                .glow
                .get_query_parameter_u32(self.id, glow::QUERY_RESULT);
            Ok(value as u64)
        }
    }

    /// Returns `QueryResultError::NotAvailable` on WebGL if the result isn't available, since
    /// WebGL can't wait for it.
    fn check_available(&self, ctxt: &mut CommandContext<'_>) -> Result<(), QueryResultError> {
        let available = || unsafe {
            ctxt.glow
                .get_query_parameter_u32(self.id, glow::QUERY_RESULT_AVAILABLE)
                != 0
        };
        if crate::version::is_webgl() && !available() {
            return Err(QueryResultError::NotAvailable);
        }
        Ok(())
    }

    unsafe fn raw_get_u64(&self, ctxt: &mut CommandContext<'_>) -> Result<u64, ()> {
        if ctxt.version.0 == Api::Gl || ctxt.extensions.gl_ext_disjoint_timer_query {
            Ok(ctxt
                .glow
                .get_query_parameter_u64(self.id, glow::QUERY_RESULT))
        } else {
            Err(())
        }
    }

    /// Returns the value of the query. Blocks until it is available.
    ///
    /// This function doesn't block if `is_ready` returns true.
    #[inline]
    pub fn get_bool(&self) -> Result<bool, QueryResultError> {
        self.get_u32().map(|value| value != 0)
    }

    /// If the query is active, unactivates it.
    fn deactivate(&self, ctxt: &mut CommandContext<'_>) {
        if ctxt.state.samples_passed_query == Some(self.id) {
            unsafe { raw_end_query(ctxt, gl::SAMPLES_PASSED) };
            ctxt.state.samples_passed_query = None;
        }

        if ctxt.state.any_samples_passed_query == Some(self.id) {
            unsafe { raw_end_query(ctxt, gl::ANY_SAMPLES_PASSED) };
            ctxt.state.any_samples_passed_query = None;
        }

        if ctxt.state.any_samples_passed_conservative_query == Some(self.id) {
            unsafe { raw_end_query(ctxt, gl::ANY_SAMPLES_PASSED_CONSERVATIVE) };
            ctxt.state.any_samples_passed_conservative_query = None;
        }

        if ctxt.state.time_elapsed_query == Some(self.id) {
            unsafe { raw_end_query(ctxt, gl::TIME_ELAPSED) };
            ctxt.state.time_elapsed_query = None;
        }

        while let Some(position) = ctxt
            .state
            .indexed_queries
            .iter()
            .position(|&(_, _, id)| id == self.id)
        {
            let (target, index, _) = ctxt.state.indexed_queries.remove(position);
            unsafe { raw_end_indexed_query(ctxt, target, index) };
        }
    }

    /// Begins the query on its target and index, which `indexed_queries` tracks.
    fn begin_indexed_query(
        &self,
        ctxt: &mut CommandContext<'_>,
        target: gl::types::GLenum,
        index: gl::types::GLuint,
    ) -> Result<(), DrawError> {
        let active = ctxt
            .state
            .indexed_queries
            .iter()
            .position(|&(t, i, _)| t == target && i == index);
        if let Some(position) = active {
            if ctxt.state.indexed_queries[position].2 == self.id {
                return Ok(());
            }
        }

        if self.has_been_used.get() {
            return Err(DrawError::WrongQueryOperation);
        }

        unsafe {
            if let Some(position) = active {
                ctxt.state.indexed_queries.remove(position);
                raw_end_indexed_query(ctxt, target, index);
            }

            if index == 0 {
                raw_begin_query(ctxt, target, self.id);
            } else {
                ctxt.extra
                    .BeginQueryIndexed(target, index, self.id.raw_name());
            }
        }

        self.mark_used(ctxt);
        ctxt.state.indexed_queries.push((target, index, self.id));
        Ok(())
    }
}

/// Makes the given primitives, pipeline statistics and transform feedback overflow queries
/// active for the next command, and ends the others: like the other queries, such a query counts
/// the commands that it is given to, as long as they follow each other. A compute dispatch only
/// gives its pipeline statistics queries (`dispatch`), and leaves the queries of primitives,
/// which it doesn't count, as they are.
///
/// Two queries of the same target and stream return `DrawError::WrongQueryOperation`.
pub(crate) fn sync_indexed_queries(
    ctxt: &mut CommandContext<'_>,
    queries: &[&RawQuery],
    dispatch: bool,
) -> Result<(), DrawError> {
    let mut targets: SmallVec<[(gl::types::GLenum, gl::types::GLuint); 4]> = SmallVec::new();
    for query in queries {
        let target = query
            .ty
            .indexed_target()
            .expect("not a primitives, pipeline statistics or overflow query");
        if targets.contains(&target) {
            return Err(DrawError::WrongQueryOperation);
        }
        targets.push(target);
    }

    let mut position = 0;
    while position < ctxt.state.indexed_queries.len() {
        let (target, index, id) = ctxt.state.indexed_queries[position];
        let primitives = target == gl::PRIMITIVES_GENERATED
            || target == gl::TRANSFORM_FEEDBACK_PRIMITIVES_WRITTEN;
        if queries.iter().any(|query| query.id == id) || (dispatch && primitives) {
            position += 1;
        } else {
            ctxt.state.indexed_queries.remove(position);
            unsafe { raw_end_indexed_query(ctxt, target, index) };
        }
    }

    for query in queries {
        query.begin_query(ctxt)?;
    }
    Ok(())
}

/// Ends the query of a target and index.
unsafe fn raw_end_indexed_query(
    ctxt: &mut CommandContext<'_>,
    target: gl::types::GLenum,
    index: gl::types::GLuint,
) {
    if index == 0 {
        raw_end_query(ctxt, target);
    } else {
        ctxt.extra.EndQueryIndexed(target, index);
    }
}

impl Drop for RawQuery {
    fn drop(&mut self) {
        let mut ctxt = self.context.make_current();
        self.deactivate(&mut ctxt);

        if let Some((id, _)) = ctxt.state.conditional_render {
            if id == self.id {
                RawQuery::end_conditional_render(&mut ctxt);
            }
        }

        unsafe {
            if !ctxt.objects_lost() {
                ctxt.glow.delete_query(self.id);
            }
        }
    }
}

impl QueryExt for RawQuery {
    fn begin_query(&self, ctxt: &mut CommandContext<'_>) -> Result<(), DrawError> {
        match self.ty {
            QueryType::SamplesPassed => {
                if ctxt.state.any_samples_passed_query.is_some() {
                    ctxt.state.any_samples_passed_query = None;
                    unsafe {
                        raw_end_query(ctxt, gl::ANY_SAMPLES_PASSED);
                    }
                }

                if ctxt.state.any_samples_passed_conservative_query.is_some() {
                    ctxt.state.any_samples_passed_conservative_query = None;
                    unsafe {
                        raw_end_query(ctxt, gl::ANY_SAMPLES_PASSED_CONSERVATIVE);
                    }
                }

                if ctxt.state.samples_passed_query != Some(self.id) {
                    if self.has_been_used.get() {
                        return Err(DrawError::WrongQueryOperation);
                    }

                    unsafe {
                        if ctxt.state.samples_passed_query.is_some() {
                            raw_end_query(ctxt, gl::SAMPLES_PASSED);
                        }
                        raw_begin_query(ctxt, gl::SAMPLES_PASSED, self.id);
                    }

                    self.mark_used(ctxt);
                    ctxt.state.samples_passed_query = Some(self.id);
                }
            }

            QueryType::AnySamplesPassed => {
                if ctxt.state.samples_passed_query.is_some() {
                    ctxt.state.samples_passed_query = None;
                    unsafe {
                        raw_end_query(ctxt, gl::SAMPLES_PASSED);
                    }
                }

                if ctxt.state.any_samples_passed_conservative_query.is_some() {
                    ctxt.state.any_samples_passed_conservative_query = None;
                    unsafe {
                        raw_end_query(ctxt, gl::ANY_SAMPLES_PASSED_CONSERVATIVE);
                    }
                }

                if ctxt.state.any_samples_passed_query != Some(self.id) {
                    if self.has_been_used.get() {
                        return Err(DrawError::WrongQueryOperation);
                    }

                    unsafe {
                        if ctxt.state.any_samples_passed_query.is_some() {
                            raw_end_query(ctxt, gl::ANY_SAMPLES_PASSED);
                        }
                        raw_begin_query(ctxt, gl::ANY_SAMPLES_PASSED, self.id);
                    }

                    self.mark_used(ctxt);
                    ctxt.state.any_samples_passed_query = Some(self.id);
                }
            }

            QueryType::AnySamplesPassedConservative => {
                if ctxt.state.samples_passed_query.is_some() {
                    ctxt.state.samples_passed_query = None;
                    unsafe {
                        raw_end_query(ctxt, gl::SAMPLES_PASSED);
                    }
                }

                if ctxt.state.any_samples_passed_query.is_some() {
                    ctxt.state.any_samples_passed_query = None;
                    unsafe {
                        raw_end_query(ctxt, gl::ANY_SAMPLES_PASSED);
                    }
                }

                if ctxt.state.any_samples_passed_conservative_query != Some(self.id) {
                    if self.has_been_used.get() {
                        return Err(DrawError::WrongQueryOperation);
                    }

                    unsafe {
                        if ctxt.state.any_samples_passed_conservative_query.is_some() {
                            raw_end_query(ctxt, gl::ANY_SAMPLES_PASSED_CONSERVATIVE);
                        }
                        raw_begin_query(ctxt, gl::ANY_SAMPLES_PASSED_CONSERVATIVE, self.id);
                    }

                    self.mark_used(ctxt);
                    ctxt.state.any_samples_passed_conservative_query = Some(self.id);
                }
            }

            QueryType::TimeElapsed => {
                if ctxt.state.time_elapsed_query != Some(self.id) {
                    if self.has_been_used.get() {
                        return Err(DrawError::WrongQueryOperation);
                    }

                    unsafe {
                        if ctxt.state.time_elapsed_query.is_some() {
                            raw_end_query(ctxt, gl::TIME_ELAPSED);
                        }
                        raw_begin_query(ctxt, gl::TIME_ELAPSED, self.id);
                    }

                    self.mark_used(ctxt);
                    ctxt.state.time_elapsed_query = Some(self.id);
                }
            }

            QueryType::Timestamp => panic!(),

            QueryType::PrimitivesGenerated(_)
            | QueryType::TransformFeedbackPrimitivesWritten(_) => {
                let (target, index) = self.ty.indexed_target().unwrap();
                self.begin_indexed_query(ctxt, target, index)?;
            }

            #[cfg(safegl_gl46)]
            QueryType::PipelineStatistic(_)
            | QueryType::TransformFeedbackOverflow
            | QueryType::TransformFeedbackStreamOverflow(_) => {
                let (target, index) = self.ty.indexed_target().unwrap();
                self.begin_indexed_query(ctxt, target, index)?;
            }
        };

        Ok(())
    }

    fn end_samples_passed_query(ctxt: &mut CommandContext<'_>) {
        if ctxt.state.samples_passed_query.is_some() {
            ctxt.state.samples_passed_query = None;
            unsafe {
                raw_end_query(ctxt, gl::SAMPLES_PASSED);
            }
        }

        if ctxt.state.any_samples_passed_query.is_some() {
            ctxt.state.any_samples_passed_query = None;
            unsafe {
                raw_end_query(ctxt, gl::ANY_SAMPLES_PASSED);
            }
        }

        if ctxt.state.any_samples_passed_conservative_query.is_some() {
            ctxt.state.any_samples_passed_conservative_query = None;
            unsafe {
                raw_end_query(ctxt, gl::ANY_SAMPLES_PASSED_CONSERVATIVE);
            }
        }
    }

    #[inline]
    fn end_time_elapsed_query(ctxt: &mut CommandContext<'_>) {
        if ctxt.state.time_elapsed_query.is_some() {
            ctxt.state.time_elapsed_query = None;
            unsafe {
                raw_end_query(ctxt, gl::TIME_ELAPSED);
            }
        }
    }

    #[cfg(safegl_desktop)]
    fn begin_conditional_render(
        &self,
        ctxt: &mut CommandContext<'_>,
        wait: bool,
        per_region: bool,
        inverted: bool,
    ) -> Result<(), DrawError> {
        // conditional rendering is core in OpenGL 3.0, but OpenGL ES and WebGL need
        // GL_NV_conditional_render
        if ctxt.version.0 != Api::Gl && !ctxt.extensions.gl_nv_conditional_render {
            return Err(DrawError::ConditionalRenderingNotSupported);
        }
        #[cfg(safegl_gl45)]
        if inverted
            && !(ctxt.version >= &Version(Api::Gl, 4, 5)
                || ctxt.extensions.gl_arb_conditional_render_inverted)
        {
            return Err(DrawError::ConditionalRenderingInvertedNotSupported);
        }

        let new_mode = match (wait, per_region, inverted) {
            (true, true, false) => gl::QUERY_BY_REGION_WAIT,
            (true, false, false) => gl::QUERY_WAIT,
            (false, true, false) => gl::QUERY_BY_REGION_NO_WAIT,
            (false, false, false) => gl::QUERY_NO_WAIT,
            (true, true, true) => gl::QUERY_BY_REGION_WAIT_INVERTED,
            (true, false, true) => gl::QUERY_WAIT_INVERTED,
            (false, true, true) => gl::QUERY_BY_REGION_NO_WAIT_INVERTED,
            (false, false, true) => gl::QUERY_NO_WAIT_INVERTED,
        };

        // returning if the active conditional rendering is already good
        if let Some((old_id, old_mode)) = ctxt.state.conditional_render {
            if old_id == self.id {
                // if the new mode is "no_wait" but he old mode is "wait",
                // then we don't need to change it
                match (new_mode, old_mode) {
                    (a, b) if a == b => return Ok(()),
                    (gl::QUERY_NO_WAIT, gl::QUERY_WAIT) => return Ok(()),
                    (gl::QUERY_BY_REGION_NO_WAIT, gl::QUERY_BY_REGION_WAIT) => return Ok(()),
                    (gl::QUERY_NO_WAIT_INVERTED, gl::QUERY_WAIT_INVERTED) => return Ok(()),
                    (gl::QUERY_BY_REGION_NO_WAIT_INVERTED, gl::QUERY_BY_REGION_WAIT_INVERTED) => {
                        return Ok(())
                    }
                    _ => (),
                }
            }
        }

        // de-activating the existing conditional render first
        if ctxt.state.conditional_render.is_some() {
            RawQuery::end_conditional_render(ctxt);
        }

        // de-activating the query
        self.deactivate(ctxt);

        // activating
        if ctxt.version.0 == Api::Gl {
            unsafe {
                ctxt.extra
                    .BeginConditionalRender(self.id.raw_name(), new_mode)
            };
        } else if ctxt.extensions.gl_nv_conditional_render {
            unsafe {
                ctxt.extra
                    .BeginConditionalRenderNV(self.id.raw_name(), new_mode)
            };
        } else {
            unreachable!();
        }

        ctxt.state.conditional_render = Some((self.id, new_mode));
        Ok(())
    }

    fn end_conditional_render(ctxt: &mut CommandContext<'_>) {
        if ctxt.state.conditional_render.is_none() {
            return;
        }

        if ctxt.version.0 == Api::Gl {
            unsafe {
                ctxt.extra.EndConditionalRender();
            }
        } else if ctxt.extensions.gl_nv_conditional_render {
            unsafe {
                ctxt.extra.EndConditionalRenderNV();
            }
        } else {
            unreachable!();
        }

        ctxt.state.conditional_render = None;
    }

    fn is_unused(&self) -> bool {
        !self.has_been_used.get()
    }

    fn get_context(&self) -> &Context {
        &self.context
    }
}

impl fmt::Debug for RawQuery {
    #[inline]
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(fmt, "Query object {:?}", self.id)
    }
}

#[cfg(any(safegl_gl43, safegl_gles32))]
impl crate::debug::sealed::Labeled for RawQuery {
    fn labeled_object(
        &self,
    ) -> (&crate::context::Context, crate::debug::sealed::LabeledObject) {
        (
            &self.context,
            crate::debug::sealed::LabeledObject::Object(crate::gl::QUERY, self.id.0.get()),
        )
    }

    fn pending_label(&self) -> Option<&std::cell::RefCell<Option<String>>> {
        if self.has_been_used.get() {
            None
        } else {
            Some(&self.pending_label)
        }
    }
}

impl GlObject for RawQuery {
    type Id = glow::Query;

    #[inline]
    fn get_id(&self) -> glow::Query {
        self.id
    }
}

/// Calls `glBeginQuery`.
///
/// # Unsafe
///
/// The type of query must be guaranteed to be supported by the backend.
/// The id of the query must be valid.
///
unsafe fn raw_begin_query(ctxt: &mut CommandContext<'_>, ty: gl::types::GLenum, id: glow::Query) {
    ctxt.glow.begin_query(ty, id);
}

/// Calls `glEndQuery`.
///
/// # Unsafe
///
/// The type of query must be guaranteed to be supported by the backend.
unsafe fn raw_end_query(ctxt: &mut CommandContext<'_>, ty: gl::types::GLenum) {
    ctxt.glow.end_query(ty);
}

macro_rules! impl_helper {
    ($name:ident, $ret:ty, $get_fn:ident) => {
        impl $name {
            /// Queries the counter to see if the result is already available.
            ///
            /// Returns `false` if the context has been lost: the result will never be available.
            #[inline]
            pub fn is_ready(&self) -> bool {
                self.query.is_ready()
            }

            /// Returns the value of the query. Blocks until it is available.
            ///
            /// This function doesn't block if `is_ready` would return true.
            ///
            /// Note that you are strongly discouraged from calling this in the middle of the
            /// rendering process, as it may block for a long time.
            ///
            /// Queries should either have their result written into a buffer, be used for
            /// conditional rendering, or stored and checked during the next frame.
            ///
            /// Returns an error if the OpenGL context has been lost, since the result is lost
            /// with it, and, on WebGL, `QueryResultError::NotAvailable` while `is_ready` returns
            /// false: WebGL can't wait for the result, which it makes available in a later task
            /// of the browser's event loop.
            #[inline]
            pub fn get(&self) -> Result<$ret, QueryResultError> {
                self.query.$get_fn()
            }

            /// Writes the result of the query to a buffer when it is available.
            ///
            /// This function doesn't block. Instead it submits a commands to the GPU's commands
            /// queue and orders the GPU to write the result of the query to a buffer.
            ///
            /// This operation is not necessarily supported everywhere.
            #[cfg(safegl_gl44)]
            #[cfg_attr(docsrs, doc(cfg(safegl_gl44)))]
            #[inline]
            pub fn to_buffer_u32(&self, target: BufferSlice<'_, u32>) -> Result<(), ToBufferError> {
                self.query.write_to_buffer(target, QueryBufferMode::Result)
            }

            /// Writes the result of the query, or its availability, to a buffer of `u32`, `i32`,
            /// `u64` or `i64`, without blocking: the GPU writes it when it runs the command.
            ///
            /// Needs OpenGL 4.4, `GL_ARB_query_buffer_object` or `GL_AMD_query_buffer_object`.
            #[cfg(safegl_gl44)]
            #[cfg_attr(docsrs, doc(cfg(safegl_gl44)))]
            #[inline]
            pub fn to_buffer<T: QueryBufferValue>(
                &self,
                target: BufferSlice<'_, T>,
                mode: QueryBufferMode,
            ) -> Result<(), ToBufferError> {
                self.query.write_to_buffer(target, mode)
            }
        }

        #[cfg(any(safegl_gl43, safegl_gles32))]
        impl crate::debug::sealed::Labeled for $name {
            fn labeled_object(
                &self,
            ) -> (&crate::context::Context, crate::debug::sealed::LabeledObject) {
                crate::debug::sealed::Labeled::labeled_object(&self.query)
            }

            fn pending_label(&self) -> Option<&std::cell::RefCell<Option<String>>> {
                crate::debug::sealed::Labeled::pending_label(&self.query)
            }
        }

        impl GlObject for $name {
            type Id = glow::Query;

            #[inline]
            fn get_id(&self) -> glow::Query {
                self.query.get_id()
            }
        }

        impl QueryExt for $name {
            #[inline]
            fn begin_query(&self, ctxt: &mut CommandContext<'_>) -> Result<(), DrawError> {
                self.query.begin_query(ctxt)
            }

            #[inline]
            fn end_samples_passed_query(ctxt: &mut CommandContext<'_>) {
                RawQuery::end_samples_passed_query(ctxt)
            }

            #[inline]
            fn end_time_elapsed_query(ctxt: &mut CommandContext<'_>) {
                RawQuery::end_time_elapsed_query(ctxt)
            }

            #[inline]
            #[cfg(safegl_desktop)]
            fn begin_conditional_render(
                &self,
                ctxt: &mut CommandContext<'_>,
                wait: bool,
                per_region: bool,
                inverted: bool,
            ) -> Result<(), DrawError> {
                self.query
                    .begin_conditional_render(ctxt, wait, per_region, inverted)
            }

            #[inline]
            fn end_conditional_render(ctxt: &mut CommandContext<'_>) {
                RawQuery::end_conditional_render(ctxt)
            }

            #[inline]
            fn is_unused(&self) -> bool {
                self.query.is_unused()
            }

            #[inline]
            fn get_context(&self) -> &Context {
                QueryExt::get_context(&self.query)
            }
        }
    };
}

/// A query that allows you to know the number of samples written to the output during the
/// draw operations where this query was active.
///
/// If you just want to know whether or not some samples have been written, you should use
/// a `AnySamplesPassedQuery` query instead.
#[derive(Debug)]
#[cfg(safegl_desktop)]
#[cfg_attr(docsrs, doc(cfg(safegl_desktop)))]
pub struct SamplesPassedQuery {
    query: RawQuery,
}

#[cfg(safegl_desktop)]
impl SamplesPassedQuery {
    /// Builds a new query.
    #[inline]
    pub fn new<F: ?Sized>(facade: &F) -> Result<SamplesPassedQuery, QueryCreationError>
    where
        F: Facade,
    {
        RawQuery::new(facade, QueryType::SamplesPassed).map(|q| SamplesPassedQuery { query: q })
    }
}

#[cfg(safegl_desktop)]
impl_helper!(SamplesPassedQuery, u64, get_u64);

/// A query that allows you to know the number of nanoseconds that have elapsed
/// during the draw operations.
///
/// The result is 64 bits wide, since 32 bits of nanoseconds overflow after 4.3 seconds.
///
/// OpenGL ES needs `GL_EXT_disjoint_timer_query`, whose results are meaningless when the GPU
/// changed its frequency or was reset during the query (`GL_GPU_DISJOINT_EXT`).
#[derive(Debug)]
#[cfg(safegl_desktop)]
#[cfg_attr(docsrs, doc(cfg(safegl_desktop)))]
pub struct TimeElapsedQuery {
    query: RawQuery,
}

#[cfg(safegl_desktop)]
impl TimeElapsedQuery {
    /// Builds a new query.
    #[inline]
    pub fn new<F: ?Sized>(facade: &F) -> Result<TimeElapsedQuery, QueryCreationError>
    where
        F: Facade,
    {
        RawQuery::new(facade, QueryType::TimeElapsed).map(|q| TimeElapsedQuery { query: q })
    }
}

#[cfg(safegl_desktop)]
impl_helper!(TimeElapsedQuery, u64, get_u64);

/// A query type that allows you to know whether any sample has been written to the output during
/// the operations executed with this query.
///
/// ## OpenGL
///
/// This is usually a query of type `GL_ANY_SAMPLES_PASSED` or
/// `GL_ANY_SAMPLES_PASSED_CONSERVATIVE`.
///
/// However if the backend doesn't support conservative queries, SafeGL will automatically fall
/// back to a non-conservative query. If the backend doesn't support either types but supports
/// `GL_SAMPLES_PASSED`, then SafeGL will automatically use a `GL_SAMPLES_PASSED` query instead.
#[derive(Debug)]
pub struct AnySamplesPassedQuery {
    query: RawQuery,
}

impl AnySamplesPassedQuery {
    /// Builds a new query.
    ///
    /// If you pass `true` for `conservative`, then OpenGL may use a less accurate algorithm,
    /// leading to a faster result but with more false positives.
    pub fn new<F: ?Sized>(
        facade: &F,
        conservative: bool,
    ) -> Result<AnySamplesPassedQuery, QueryCreationError>
    where
        F: Facade,
    {
        if conservative {
            if let Ok(q) = RawQuery::new(facade, QueryType::AnySamplesPassedConservative) {
                return Ok(AnySamplesPassedQuery { query: q });
            }
        }

        if let Ok(q) = RawQuery::new(facade, QueryType::AnySamplesPassed) {
            Ok(AnySamplesPassedQuery { query: q })
        } else if let Ok(q) = RawQuery::new(facade, QueryType::SamplesPassed) {
            Ok(AnySamplesPassedQuery { query: q })
        } else {
            Err(QueryCreationError::NotSupported)
        }
    }
}

impl_helper!(AnySamplesPassedQuery, bool, get_bool);

/// A query that counts a statistic of the pipeline, such as the invocations of the fragment
/// shader, during the draws and the dispatches that it is given to
/// (`DrawParameters::pipeline_statistics_queries`, `ComputeShader::execute_with_statistics`).
///
/// Needs OpenGL 4.6 or `GL_ARB_pipeline_statistics_query`, otherwise `new` returns
/// `QueryCreationError::NotSupported`.
#[cfg(safegl_gl46)]
#[cfg_attr(docsrs, doc(cfg(safegl_gl46)))]
#[derive(Debug)]
pub struct PipelineStatisticsQuery {
    query: RawQuery,
    statistic: PipelineStatistic,
}

#[cfg(safegl_gl46)]
impl PipelineStatisticsQuery {
    /// Builds a new query of `statistic`.
    #[inline]
    pub fn new<F: ?Sized>(
        facade: &F,
        statistic: PipelineStatistic,
    ) -> Result<PipelineStatisticsQuery, QueryCreationError>
    where
        F: Facade,
    {
        RawQuery::new(facade, QueryType::PipelineStatistic(statistic))
            .map(|query| PipelineStatisticsQuery { query, statistic })
    }

    /// The statistic that this query counts.
    #[inline]
    pub fn statistic(&self) -> PipelineStatistic {
        self.statistic
    }

    #[inline]
    pub(crate) fn raw(&self) -> &RawQuery {
        &self.query
    }
}

#[cfg(safegl_gl46)]
impl_helper!(PipelineStatisticsQuery, u64, get_u64);

/// A query that tells whether transform feedback overflowed during the draws that it is given
/// to (`DrawParameters::transform_feedback_overflow_query`): whether a draw wrote fewer
/// primitives than it generated because the buffers of the session were full. It can also be
/// the condition of a draw.
///
/// Needs OpenGL 4.6 or `GL_ARB_transform_feedback_overflow_query`, otherwise `new` returns
/// `QueryCreationError::NotSupported`.
#[cfg(safegl_gl46)]
#[cfg_attr(docsrs, doc(cfg(safegl_gl46)))]
#[derive(Debug)]
pub struct TransformFeedbackOverflowQuery {
    query: RawQuery,
}

#[cfg(safegl_gl46)]
impl TransformFeedbackOverflowQuery {
    /// Builds a new query of the overflows of any vertex stream.
    #[inline]
    pub fn new<F: ?Sized>(facade: &F) -> Result<TransformFeedbackOverflowQuery, QueryCreationError>
    where
        F: Facade,
    {
        RawQuery::new(facade, QueryType::TransformFeedbackOverflow)
            .map(|query| TransformFeedbackOverflowQuery { query })
    }

    /// Builds a new query of the overflows of the vertex stream `stream`, which must be below
    /// `GL_MAX_VERTEX_STREAMS`, otherwise it returns `QueryCreationError::NotSupported`.
    #[inline]
    pub fn with_stream<F: ?Sized>(
        facade: &F,
        stream: u32,
    ) -> Result<TransformFeedbackOverflowQuery, QueryCreationError>
    where
        F: Facade,
    {
        RawQuery::new(facade, QueryType::TransformFeedbackStreamOverflow(stream))
            .map(|query| TransformFeedbackOverflowQuery { query })
    }

    #[inline]
    pub(crate) fn raw(&self) -> &RawQuery {
        &self.query
    }
}

#[cfg(safegl_gl46)]
impl_helper!(TransformFeedbackOverflowQuery, bool, get_bool);

/// Query that allows you to know the number of primitives generated by the last vertex
/// processing stage: the geometry shader if there is one, otherwise the tessellation evaluation
/// shader, otherwise the vertex shader (two triangles for a quad drawn as a triangle list).
///
/// OpenGL ES needs version 3.2, `GL_EXT_geometry_shader` or `GL_OES_geometry_shader`.
#[derive(Debug)]
#[cfg(any(safegl_desktop, safegl_gles32))]
#[cfg_attr(docsrs, doc(cfg(any(safegl_desktop, safegl_gles32))))]
pub struct PrimitivesGeneratedQuery {
    query: RawQuery,
}

#[cfg(any(safegl_desktop, safegl_gles32))]
impl PrimitivesGeneratedQuery {
    /// Builds a new query.
    #[inline]
    pub fn new<F: ?Sized>(facade: &F) -> Result<PrimitivesGeneratedQuery, QueryCreationError>
    where
        F: Facade,
    {
        RawQuery::new(facade, QueryType::PrimitivesGenerated(0))
            .map(|q| PrimitivesGeneratedQuery { query: q })
    }

    /// Builds a query of the primitives generated for a vertex stream of geometry shaders,
    /// below `Limits::max_vertex_streams` (`glBeginQueryIndexed`).
    #[cfg(safegl_desktop)]
    #[cfg_attr(docsrs, doc(cfg(safegl_desktop)))]
    #[inline]
    pub fn with_stream<F: ?Sized>(
        facade: &F,
        stream: u32,
    ) -> Result<PrimitivesGeneratedQuery, QueryCreationError>
    where
        F: Facade,
    {
        RawQuery::new(facade, QueryType::PrimitivesGenerated(stream))
            .map(|q| PrimitivesGeneratedQuery { query: q })
    }
    #[inline]
    pub(crate) fn raw(&self) -> &RawQuery {
        &self.query
    }
}

#[cfg(any(safegl_desktop, safegl_gles32))]
impl_helper!(PrimitivesGeneratedQuery, u64, get_u64);

/// Query that allows you to know the number of primitives generated by transform feedback.
#[derive(Debug)]
pub struct TransformFeedbackPrimitivesWrittenQuery {
    query: RawQuery,
}

impl TransformFeedbackPrimitivesWrittenQuery {
    /// Builds a new query.
    #[inline]
    pub fn new<F: ?Sized>(
        facade: &F,
    ) -> Result<TransformFeedbackPrimitivesWrittenQuery, QueryCreationError>
    where
        F: Facade,
    {
        RawQuery::new(facade, QueryType::TransformFeedbackPrimitivesWritten(0))
            .map(|q| TransformFeedbackPrimitivesWrittenQuery { query: q })
    }

    /// Builds a query of the primitives written by transform feedback for a vertex stream of
    /// geometry shaders, below `Limits::max_vertex_streams` (`glBeginQueryIndexed`).
    #[cfg(safegl_desktop)]
    #[cfg_attr(docsrs, doc(cfg(safegl_desktop)))]
    #[inline]
    pub fn with_stream<F: ?Sized>(
        facade: &F,
        stream: u32,
    ) -> Result<TransformFeedbackPrimitivesWrittenQuery, QueryCreationError>
    where
        F: Facade,
    {
        RawQuery::new(facade, QueryType::TransformFeedbackPrimitivesWritten(stream))
            .map(|q| TransformFeedbackPrimitivesWrittenQuery { query: q })
    }
    #[inline]
    pub(crate) fn raw(&self) -> &RawQuery {
        &self.query
    }
}

impl_helper!(TransformFeedbackPrimitivesWrittenQuery, u64, get_u64);
