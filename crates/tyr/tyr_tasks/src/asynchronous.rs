use std::future::Future;

use tokio::runtime::{Handle, Runtime};

use crate::{task::Task, Error, TaskSet};

/// A wrapper around the tokio runtime.
pub struct TokioRuntime(Runtime);

impl TokioRuntime {
    pub(crate) fn new(runtime: Runtime) -> Self {
        Self(runtime)
    }

    /// Returns a raw tokio [`Handle`] to the underlying runtime
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

    /// Return a handle to the underlying tokio runtime
    pub fn handle(&self) -> &Handle {
        &self.runtime_handle
    }

    /// Tries to spawn the future on the async runtime and sets the task to be alive.
    ///
    /// # Errors
    /// If the task is already alive, the future is not spawned and an [`Error::AlreadyDispatched`]
    /// is returned instead.
    ///
    /// # Example
    /// ```
    /// use tyr::{prelude::*, tasks::{AsyncDispatcher, Task, Error}};
    /// use miette::Result;
    ///
    /// async fn download_money() -> i32 {
    ///     // Needs to connect to a secret bank across the globe... this may take a while!
    ///     // ...
    ///     // After downloading we get 20 money! Nice!
    ///     20
    /// }
    ///
    /// #[system]
    /// fn get_money(dispatcher: &AsyncDispatcher, task: &mut Task<i32>) -> Result<()> {
    ///     // Dispatches the future returned by `download_money` to a background thread
    ///     // where it can be efficiently awaited without blocking all the other systems
    ///     // and tasks.
    ///     //
    ///     // Also marks the task as `alive`, so we can't accidentally dispatch it twice.
    ///     match dispatcher.try_dispatch(&mut task, download_money()) {
    ///         // Successfully dispatched the task
    ///         Ok(_) => Ok(()),
    ///         // This is also fine here, we are already running the task and can continue
    ///         // without dispatching it again
    ///         Err(Error::AlreadyDispatched) => Ok(()),
    ///     }
    /// }
    ///
    /// #[system]
    /// fn handle_completion(
    ///     task: &mut Task<i32>,
    /// ) -> Result<()> {
    ///     let Some(money) = task.poll() else {
    ///         // Task is not yet ready, return early!
    ///         return Ok(());
    ///     };
    ///     // Our task has completed! We can now use `money`!
    ///     Ok(())
    /// }
    /// ```
    pub fn try_dispatch<F: Future + Send + 'static>(
        &self,
        task: &mut Task<F::Output>,
        future: F,
    ) -> crate::Result<()>
    where
        F::Output: Send,
    {
        if task.is_alive() {
            Err(Error::AlreadyDispatched)
        } else {
            task.join_handle = Some(self.runtime_handle.spawn(future));
            Ok(())
        }
    }

    /// Dispatches a task onto a `[TaskSet<T>]`
    pub fn dispatch_set<F: Future + Send + 'static>(
        &self,
        task_set: &mut TaskSet<F::Output>,
        future: F,
    ) where
        F::Output: Send + Unpin,
    {
        let _guard = self.runtime_handle.enter();
        task_set.join_set.spawn(future);
    }
}
