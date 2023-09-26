mod asynchronous;
mod compute;
mod task;

use std::sync::Arc;

use compute::RayonThreadPool;
use miette::{Diagnostic, IntoDiagnostic, Result as MietteResult};
use rayon::ThreadPoolBuilder;
use thiserror::Error;
use tokio::runtime;
use tyr_internal::{App, Module, Resource};

use crate::asynchronous::TokioRuntime;

pub use crate::asynchronous::{AsyncDispatcher, AsyncTask, AsyncTaskMap, AsyncTaskSet};
pub use crate::compute::{ComputeDispatcher, ComputeTask, ComputeTaskMap, ComputeTaskSet};
pub use crate::task::{Task, TaskMap, TaskResource, TaskSet};

// TODO: customisable async/compute thread count through config

/// [`Module`](../../tyr/trait.Module.html) implementing asynchronous task execution.
///
/// Adds the following usable [`Resource`]s to the [`App`]:
///  - [`AsyncDispatcher`]
///  - [`ComputeDispatcher`]
///
/// Some functions may block the main thread for a long time.
/// Examples where this can happen include waiting on network messages, processing camera data or running big machine learning models.
///
/// Use an [`AsyncDispatcher`] or [`ComputeDispatcher`] to run asynchronous or compute heavy functions over multiple execution cycles.
pub struct TaskModule;

impl Module for TaskModule {
    fn initialize(self, app: App) -> MietteResult<App> {
        let runtime = TokioRuntime::new(
            runtime::Builder::new_multi_thread()
                .worker_threads(1)
                .thread_name("tokio-async-worker")
                .enable_all()
                .build()
                .into_diagnostic()?,
        );

        let async_dispatcher = AsyncDispatcher::new(runtime.handle().clone());

        let thread_pool = RayonThreadPool::new(Arc::new(
            ThreadPoolBuilder::new()
                .num_threads(2)
                .thread_name(|idx| format!("rayon-compute-worker-{idx}"))
                .build()
                .into_diagnostic()?,
        ));

        let compute_dispatcher =
            ComputeDispatcher::new(thread_pool.clone(), async_dispatcher.clone());

        app.add_resource(Resource::new(runtime))?
            .add_resource(Resource::new(thread_pool))?
            .add_resource(Resource::new(async_dispatcher))?
            .add_resource(Resource::new(compute_dispatcher))
    }
}

/// A specialized [`Result`] type returning an [`tyr::tasks::Error`](`enum@Error`)
pub type Result<T> = std::result::Result<T, Error>;

/// The error type for task operations
#[derive(Debug, Error, Diagnostic)]
pub enum Error {
    #[error("Task is already dispatched")]
    AlreadyAlive,
}
