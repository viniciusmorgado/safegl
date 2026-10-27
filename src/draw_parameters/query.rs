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
}

pub enum QueryType {
    SamplesPassed,
    AnySamplesPassed,
    AnySamplesPassedConservative,
    TimeElapsed,
    Timestamp,
    PrimitivesGenerated,
    TransformFeedbackPrimitivesWritten,
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
            QueryType::PrimitivesGenerated => gl::PRIMITIVES_GENERATED,
            QueryType::TransformFeedbackPrimitivesWritten => {
                gl::TRANSFORM_FEEDBACK_PRIMITIVES_WRITTEN
            }
        }
    }
}

/// Error that can happen when creating a query object.
#[derive(Copy, Clone, Debug)]
pub enum QueryCreationError {
    /// The given query type is not supported.
    NotSupported,
}

impl fmt::Display for QueryCreationError {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        use self::QueryCreationError::*;
        let desc = match *self {
            NotSupported => "The given query type is not supported",
        };
        fmt.write_str(desc)
    }
}

impl Error for QueryCreationError {}

/// Error that can happen when writing the value of a query to a buffer.
#[cfg(safegl_gl44)]
#[cfg_attr(docsrs, doc(cfg(safegl_gl44)))]
#[derive(Copy, Clone, Debug)]
pub enum ToBufferError {
    /// Writing the result to a buffer is not supported.
    NotSupported,
}

#[cfg(safegl_gl44)]
impl fmt::Display for ToBufferError {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        use self::ToBufferError::*;
        let desc = match *self {
            NotSupported => "Writing the result to a buffer is not supported",
        };
        fmt.write_str(desc)
    }
}

#[cfg(safegl_gl44)]
impl Error for ToBufferError {}

/// Error that can happen when getting the result of a query.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum QueryResultError {
    /// The OpenGL context has been lost, so the result of the query is lost too.
    ContextLost,
}

impl fmt::Display for QueryResultError {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        let desc = match *self {
            QueryResultError::ContextLost => {
                "The OpenGL context has been lost, so the result of the query is lost too"
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

        // FIXME: handle Timestamp separately

        let id = unsafe {
            if ctxt.version.0 == Api::Gl {
                match ty {
                    QueryType::AnySamplesPassed
                    | QueryType::SamplesPassed
                    | QueryType::PrimitivesGenerated
                    | QueryType::TimeElapsed
                    | QueryType::TransformFeedbackPrimitivesWritten => (),
                    QueryType::AnySamplesPassedConservative
                        if ctxt.extensions.gl_arb_es3_compatibility
                            || ctxt.version >= &Version(Api::Gl, 4, 3) =>
                    {
                        ()
                    }
                    _ => return Err(QueryCreationError::NotSupported),
                };

                ctxt.glow.create_query().expect("glGenQueries failed")
            } else {
                // OpenGL ES 3.0, plus the queries of geometry shaders and of
                // `GL_EXT_disjoint_timer_query`
                match ty {
                    QueryType::AnySamplesPassed
                    | QueryType::AnySamplesPassedConservative
                    | QueryType::TransformFeedbackPrimitivesWritten => (),
                    QueryType::PrimitivesGenerated
                        if ctxt.version >= &Version(Api::GlEs, 3, 2)
                            || ctxt.extensions.gl_ext_geometry_shader
                            || ctxt.extensions.gl_oes_geometry_shader => {}
                    QueryType::TimeElapsed if ctxt.extensions.gl_ext_disjoint_timer_query => {}
                    _ => return Err(QueryCreationError::NotSupported),
                };

                ctxt.glow.create_query().expect("glGenQueries failed")
            }
        };

        Ok(RawQuery {
            context,
            id,
            ty,
            has_been_used: Cell::new(false),
        })
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

        if !self.has_been_used.get() {
            return false;
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

        unsafe {
            Ok(ctxt
                .glow
                .get_query_parameter_u32(self.id, glow::QUERY_RESULT))
        }
    }

    /// Writes the value of the query to a buffer.
    #[cfg(safegl_gl44)]
    pub fn write_u32_to_buffer(&self, target: BufferSlice<'_, u32>) -> Result<(), ToBufferError> {
        let mut ctxt = self.context.make_current();

        if !(ctxt.version >= &Version(Api::Gl, 4, 4)
            || ctxt.extensions.gl_arb_query_buffer_object
            || ctxt.extensions.gl_amd_query_buffer_object)
        {
            return Err(ToBufferError::NotSupported);
        }

        self.deactivate(&mut ctxt);

        if !self.has_been_used.get() {
            // the result of a query that was never used is 0, like `get` returns, but OpenGL
            // refuses to write the result of a query object that never started
            drop(ctxt);
            target.write(&0);
            return Ok(());
        }

        assert!(target.get_offset_bytes() % 4 == 0);

        target.prepare_and_bind_for_query(&mut ctxt);
        unsafe {
            self.raw_get_u32(&mut ctxt, target.get_offset_bytes() as *mut _);
        }

        if let Some(fence) = target.add_fence() {
            fence.insert(&mut ctxt);
        }

        Ok(())
    }

    unsafe fn raw_get_u32(&self, ctxt: &mut CommandContext<'_>, target: *mut gl::types::GLuint) {
        ctxt.extra
            .GetQueryObjectuiv(self.id.raw_name(), gl::QUERY_RESULT, target);
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

        if ctxt.state.primitives_generated_query == Some(self.id) {
            unsafe { raw_end_query(ctxt, gl::PRIMITIVES_GENERATED) };
            ctxt.state.primitives_generated_query = None;
        }

        if ctxt.state.transform_feedback_primitives_written_query == Some(self.id) {
            unsafe { raw_end_query(ctxt, gl::TRANSFORM_FEEDBACK_PRIMITIVES_WRITTEN) };
            ctxt.state.transform_feedback_primitives_written_query = None;
        }

        if ctxt.state.time_elapsed_query == Some(self.id) {
            unsafe { raw_end_query(ctxt, gl::TIME_ELAPSED) };
            ctxt.state.time_elapsed_query = None;
        }
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
            ctxt.glow.delete_query(self.id);
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

                    self.has_been_used.set(true);
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

                    self.has_been_used.set(true);
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

                    self.has_been_used.set(true);
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

                    self.has_been_used.set(true);
                    ctxt.state.time_elapsed_query = Some(self.id);
                }
            }

            QueryType::Timestamp => panic!(),

            QueryType::PrimitivesGenerated => {
                if ctxt.state.primitives_generated_query != Some(self.id) {
                    if self.has_been_used.get() {
                        return Err(DrawError::WrongQueryOperation);
                    }

                    unsafe {
                        if ctxt.state.primitives_generated_query.is_some() {
                            raw_end_query(ctxt, gl::PRIMITIVES_GENERATED);
                        }
                        raw_begin_query(ctxt, gl::PRIMITIVES_GENERATED, self.id);
                    }

                    self.has_been_used.set(true);
                    ctxt.state.primitives_generated_query = Some(self.id);
                }
            }

            QueryType::TransformFeedbackPrimitivesWritten => {
                if ctxt.state.transform_feedback_primitives_written_query != Some(self.id) {
                    if self.has_been_used.get() {
                        return Err(DrawError::WrongQueryOperation);
                    }

                    unsafe {
                        if ctxt
                            .state
                            .transform_feedback_primitives_written_query
                            .is_some()
                        {
                            raw_end_query(ctxt, gl::TRANSFORM_FEEDBACK_PRIMITIVES_WRITTEN);
                        }
                        raw_begin_query(ctxt, gl::TRANSFORM_FEEDBACK_PRIMITIVES_WRITTEN, self.id);
                    }

                    self.has_been_used.set(true);
                    ctxt.state.transform_feedback_primitives_written_query = Some(self.id);
                }
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

    #[inline]
    fn end_primitives_generated_query(ctxt: &mut CommandContext<'_>) {
        if ctxt.state.primitives_generated_query.is_some() {
            ctxt.state.primitives_generated_query = None;
            unsafe {
                raw_end_query(ctxt, gl::PRIMITIVES_GENERATED);
            }
        }
    }

    #[inline]
    fn end_transform_feedback_primitives_written_query(ctxt: &mut CommandContext<'_>) {
        if ctxt
            .state
            .transform_feedback_primitives_written_query
            .is_some()
        {
            ctxt.state.transform_feedback_primitives_written_query = None;
            unsafe {
                raw_end_query(ctxt, gl::TRANSFORM_FEEDBACK_PRIMITIVES_WRITTEN);
            }
        }
    }

    #[cfg(safegl_desktop)]
    fn begin_conditional_render(
        &self,
        ctxt: &mut CommandContext<'_>,
        wait: bool,
        per_region: bool,
    ) -> Result<(), DrawError> {
        // conditional rendering is core in OpenGL 3.0, but OpenGL ES and WebGL need
        // GL_NV_conditional_render
        if ctxt.version.0 != Api::Gl && !ctxt.extensions.gl_nv_conditional_render {
            return Err(DrawError::ConditionalRenderingNotSupported);
        }

        let new_mode = match (wait, per_region) {
            (true, true) => gl::QUERY_BY_REGION_WAIT,
            (true, false) => gl::QUERY_WAIT,
            (false, true) => gl::QUERY_BY_REGION_NO_WAIT,
            (false, false) => gl::QUERY_NO_WAIT,
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
            /// with it.
            #[inline]
            pub fn get(self) -> Result<$ret, QueryResultError> {
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
                self.query.write_u32_to_buffer(target)
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
            fn end_primitives_generated_query(ctxt: &mut CommandContext<'_>) {
                RawQuery::end_primitives_generated_query(ctxt)
            }

            #[inline]
            fn end_transform_feedback_primitives_written_query(ctxt: &mut CommandContext<'_>) {
                RawQuery::end_transform_feedback_primitives_written_query(ctxt)
            }

            #[inline]
            #[cfg(safegl_desktop)]
            fn begin_conditional_render(
                &self,
                ctxt: &mut CommandContext<'_>,
                wait: bool,
                per_region: bool,
            ) -> Result<(), DrawError> {
                self.query.begin_conditional_render(ctxt, wait, per_region)
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
        RawQuery::new(facade, QueryType::PrimitivesGenerated)
            .map(|q| PrimitivesGeneratedQuery { query: q })
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
        RawQuery::new(facade, QueryType::TransformFeedbackPrimitivesWritten)
            .map(|q| TransformFeedbackPrimitivesWrittenQuery { query: q })
    }
}

impl_helper!(TransformFeedbackPrimitivesWrittenQuery, u64, get_u64);
