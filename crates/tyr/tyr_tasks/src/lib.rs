mod asynchronous;
mod compute;
mod task;

/// Tasks allow functions to run over multiple cycles.
pub mod tasks {
    use miette::{IntoDiagnostic, Result};
    use rayon::ThreadPoolBuilder;
    use tokio::runtime;
    use tyr_internal::{App, Module, Resource};

    use crate::asynchronous::TokioRuntime;

    pub use crate::asynchronous::AsyncDispatcher;
    pub use crate::compute::ComputeDispatcher;
    pub use crate::task::{Task, TaskResource};

    // TODO: customisable async/compute thread count through config

    /// [`Module`](../../tyr/trait.Module.html) implementing asynchronous task execution.
    ///
    /// Some functions may block the main thread for a long time.
    /// Examples where this can happen include waiting on network messages, processing camera data or running big machine learning models.
    ///
    /// Use an [`AsyncDispatcher`] or [`ComputeDispatcher`] to run asynchronous or compute heavy functions over multiple execution cycles.
    pub struct TaskModule;

    impl Module for TaskModule {
        fn initialize(self, app: App) -> Result<App> {
            let runtime = TokioRuntime::new(
                runtime::Builder::new_multi_thread()
                    .worker_threads(1)
                    .thread_name("tokio-async-worker")
                    .enable_all()
                    .build()
                    .into_diagnostic()?,
            );

            let async_dispatcher = AsyncDispatcher::new(runtime.handle().clone());

            let thread_pool = ThreadPoolBuilder::new()
                .num_threads(2)
                .thread_name(|idx| format!("rayon-compute-worker-{}", idx))
                .build()
                .into_diagnostic()?;

            let compute_dispatcher = ComputeDispatcher::new(thread_pool, async_dispatcher.clone());

            app.add_resource(Resource::new(runtime))?
                .add_resource(Resource::new(async_dispatcher))?
                .add_resource(Resource::new(compute_dispatcher))
        }
    }
}
