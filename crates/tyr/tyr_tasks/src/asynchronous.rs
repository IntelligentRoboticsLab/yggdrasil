use std::{any::type_name, future::Future};

use miette::{miette, Result};
use tokio::runtime::{Handle, Runtime};

use crate::task::Task;

/// A wrapper around the tokio runtime.
pub struct TokioRuntime(Runtime);

impl TokioRuntime {
    pub fn new(runtime: Runtime) -> Self {
        Self(runtime)
    }

    pub fn handle(&self) -> &Handle {
        self.0.handle()
    }
}

/// Dispatcher that allows futures to run efficiently on a separate thread without blocking the main thread.
#[derive(Clone)]
pub struct AsyncDispatcher {
    runtime_handle: Handle,
}

impl AsyncDispatcher {
    pub(crate) fn new(runtime_handle: Handle) -> Self {
        Self { runtime_handle }
    }

    /// Spawns the future on the async runtime and sets the task to be alive.
    ///
    /// # Example
    /// ```ignore
    ///    async fn download_money() -> i32 {
    ///        // Needs to connect to a secret bank across the globe... this may take a while!
    ///        // ...
    ///        // After downloading we get 20 money! Nice!
    ///        20
    ///    }
    ///
    ///    #[system]
    ///    fn get_money(dispatcher: &AsyncDispatcher, task: &mut Task<i32>) -> Result<()> {
    ///        // Task is already running, so we can't dispatch more at this time...
    ///        if task.is_alive() {
    ///            return Ok(());
    ///        }
    ///
    ///        // Dispatches the future returned by `download_money` to a background thread
    ///        // where it can be efficiently awaited without blocking all the other systems
    ///        // and tasks.
    ///        //
    ///        // Also marks the task as `alive`, so we can't accidentally dispatch it twice.
    ///        dispatcher.dispatch(&mut task, download_money())?;
    ///
    ///        Ok(())
    ///    }
    ///
    ///    #[system]
    ///    fn handle_completion(
    ///        task: &mut Task<i32>,
    ///    ) -> Result<()> {
    ///        let Some(money) = task.poll() else {
    ///            // Task is not yet ready, return early!
    ///            return Ok(());
    ///        };
    ///        // Our task has completed! We can now use `money`!
    ///        Ok(())
    ///    }
    /// ```
    pub fn dispatch<F: Future + Send + 'static>(
        &self,
        task: &mut Task<F::Output>,
        future: F,
    ) -> Result<()>
    where
        F::Output: Send,
    {
        // TODO: is this the behaviour do we want here? Perhaps a future version could include some kind of `TaskSet` that allow for multiple dispatches?
        if task.is_alive() {
            return Err(miette!(
                "Trying to dispatch async task `{}` which is already alive!",
                type_name::<Task<F::Output>>()
            ));
        }

        let join_handle = Some(self.runtime_handle.spawn(future));

        *task = Task { join_handle };

        Ok(())
    }
}
