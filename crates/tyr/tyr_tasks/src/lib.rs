//! Tasks allow functions to run over multiple execution cycles.

pub mod asynchronous;
pub mod compute;
pub mod task;

use std::sync::Arc;

use miette::{Diagnostic, IntoDiagnostic, Result as MietteResult};
use rayon::ThreadPoolBuilder;

use thiserror::Error;
use tokio::runtime;

use tyr_internal::{App, Module, Res, Resource, Storage};

#[cfg(feature = "odal")]
use serde::{Deserialize, Serialize};

use asynchronous::TokioRuntime;
use compute::RayonThreadPool;

/// [`Module`](../../tyr/trait.Module.html) implementing asynchronous task execution.
///
/// # Warning ⚠️
/// Initialization of this module requires a [`TaskConfig`] resource to be in storage.
///
/// Adds the following usable [`Resource`]s to the [`App`]:
///  - [`AsyncDispatcher`](`asynchronous::AsyncDispatcher`)
///  - [`ComputeDispatcher`](`compute::ComputeDispatcher`)
///
/// Some functions may block the main thread for a long time.
/// Examples where this can happen include waiting on network messages, processing camera data or running big machine learning models.
/// These are ideal use cases for tasks, as they allow you to offload work to other threads. This way the robot control can keep
/// running smoothly.
#[derive(Debug)]
pub struct TaskModule;

/// Configuration for the task dispatchers
#[cfg_attr(feature = "odal", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "odal", serde(deny_unknown_fields))]
#[derive(Debug, Clone)]
pub struct TaskConfig {
    // The amount of threads the underlying tokio runtime may use
    pub async_threads: usize,
    // The amount of threads the underlying rayon threadpool may use
    pub compute_threads: usize,
}

impl Module for TaskModule {
    fn initialize(self, app: App) -> MietteResult<App> {
        app.add_startup_system(init_tasks)
    }
}

/// Initializes tasks according to the [`TaskConfig`] resource.
fn init_tasks(storage: &mut Storage, config: Res<TaskConfig>) -> MietteResult<()> {
    let runtime = TokioRuntime::new(
        runtime::Builder::new_multi_thread()
            .worker_threads(config.async_threads)
            .thread_name("tokio-async-worker")
            .enable_all()
            .build()
            .into_diagnostic()?,
    );

    let async_dispatcher = asynchronous::AsyncDispatcher::new(runtime.handle().clone());

    let thread_pool = RayonThreadPool::new(Arc::new(
        ThreadPoolBuilder::new()
            .num_threads(config.compute_threads)
            .thread_name(|idx| format!("rayon-compute-worker-{idx}"))
            .build()
            .into_diagnostic()?,
    ));

    let compute_dispatcher =
        compute::ComputeDispatcher::new(thread_pool.clone(), async_dispatcher.clone());

    storage.add_resource(Resource::new(runtime))?;
    storage.add_resource(Resource::new(thread_pool))?;
    storage.add_resource(Resource::new(async_dispatcher))?;
    storage.add_resource(Resource::new(compute_dispatcher))?;

    Ok(())
}

/// A specialized [`Result`] type returning a [`tyr::tasks::Error`](`enum@Error`).
pub type Result<T> = std::result::Result<T, Error>;

/// The error type for task operations.
#[derive(Debug, Error, Diagnostic)]
pub enum Error {
    #[error("Task is already dispatched")]
    AlreadyActive,
    #[error("Task is not dispatched")]
    NotActive,
}

/// The prelude contains commonly used items.
///
/// It is re-exported by the top-level tyr module. To import, simply type:
/// `use tyr::prelude::*;`
pub mod prelude {
    pub use crate::{
        asynchronous::{AsyncDispatcher, AsyncTask, AsyncTaskMap, AsyncTaskSet},
        compute::{ComputeDispatcher, ComputeTask, ComputeTaskMap, ComputeTaskSet},
        task::{Dispatcher, Pollable, TaskResource},
    };
}
