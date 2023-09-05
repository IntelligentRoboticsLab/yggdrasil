use std::{
    future::Future,
    panic::{catch_unwind, resume_unwind, AssertUnwindSafe},
    pin::Pin,
    task::{Context, Poll},
    thread,
};

use miette::{IntoDiagnostic, WrapErr};
use rayon::ThreadPool;
use tokio::sync::oneshot::{self, Receiver};

use crate::{asynchronous::AsyncDispatcher, task::Task, Error};

#[derive(Debug)]
pub struct ComputeJoinHandle<T> {
    rx: Receiver<thread::Result<T>>,
}

impl<T> Future for ComputeJoinHandle<T> {
    type Output = T;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let rx = Pin::new(&mut self.rx);
        rx.poll(cx).map(|result| {
            result
                .into_diagnostic()
                .wrap_err_with(|| {
                    format!(
                        "Unable to poll for `{}`",
                        std::any::type_name::<Self::Output>()
                    )
                })
                .unwrap()
                // handle caught panics by panicking from the tokio thread
                .unwrap_or_else(|e| resume_unwind(e))
        })
    }
}

/// Dispatcher that allows functions to run on a separate threadpool that uses
/// an efficient work-stealing scheduler.
pub struct ComputeDispatcher {
    thread_pool: ThreadPool,
    async_dispatcher: AsyncDispatcher,
}

impl ComputeDispatcher {
    pub(crate) fn new(thread_pool: ThreadPool, async_dispatcher: AsyncDispatcher) -> Self {
        Self {
            thread_pool,
            async_dispatcher,
        }
    }

    /// Tries to spawn the function on the threadpool and sets the task to be alive.
    ///
    /// # Errors
    /// If the task is already alive, the function is not spawned and an [`Error::AlreadyDispatched`]
    /// is returned instead.
    ///
    /// # Example
    /// ```
    /// use tyr::{prelude::*, tasks::{ComputeDispatcher, Task, Error}};
    /// use miette::Result;
    ///
    /// fn big_ass_calculation() -> i32 {
    ///      // We're gonna need a lot of resources...
    ///      // ...
    ///      // Finally, we get our processed value
    ///      21
    /// }
    ///
    /// #[system]
    /// fn do_calculation(dispatcher: &ComputeDispatcher, task: &mut Task<i32>) -> Result<()> {
    ///     // Dispatches the computation of `big_ass_calculation` to a background thread
    ///     // where it can be efficiently computed in parallel and without blocking all
    ///     // the other systems and tasks.
    ///     //
    ///     // Also marks the task as `alive`, so we can't accidentally dispatch it twice.
    ///     match dispatcher.try_dispatch(&mut task, move || big_ass_calculation()) {
    ///         // Successfully dispatched the task
    ///         Ok(_) => Ok(()),
    ///         // This is also fine here, we are already running the task and can continue
    ///         // without dispatching it again
    ///         Err(Error::AlreadyDispatched) => Ok(()),
    ///     }
    ///
    /// }
    ///
    /// #[system]
    /// fn handle_completion(
    ///     task: &mut Task<i32>,
    /// ) -> Result<()> {
    ///     let Some(value) = task.poll() else {
    ///         // Task is not yet ready, return early!
    ///         return Ok(());
    ///     };
    ///     // Our task has completed! We can now use `value`!
    ///     Ok(())
    /// }
    /// ```
    pub fn try_dispatch<F, T>(&self, task: &mut Task<T>, func: F) -> crate::Result<()>
    where
        F: FnOnce() -> T + Send + 'static,
        T: Send + 'static,
    {
        if task.is_alive() {
            return Err(Error::AlreadyDispatched);
        }

        let (tx, rx) = oneshot::channel();
        self.thread_pool.spawn(move || {
            // send the result of invoking the function back through the oneshot channel,
            // capturing any panics that might occur
            let _result = tx.send(catch_unwind(AssertUnwindSafe(func)));
        });

        let compute_join_handle = ComputeJoinHandle { rx };

        self.async_dispatcher.dispatch(task, compute_join_handle);

        Ok(())
    }
}
