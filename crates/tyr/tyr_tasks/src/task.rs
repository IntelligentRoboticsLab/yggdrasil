use futures_lite::future;
use miette::{IntoDiagnostic, Result, WrapErr};
use tokio::task::JoinHandle;

use tyr_internal::{App, Resource};

/// Tasks allow functions to complete after multiple execution cycles.
///
/// Tasks might be in either a dead or alive state.
/// - A task is alive when the value of `T` is being awaited or calculated.
/// - A task is dead when there is nothing to be awaited or calculated.
///
/// You can check if it is alive using the [`Task::is_alive`] method.
///
/// To get the value out of a task, you must check it's completion using the [`Task::poll`] method.
///
/// To activate a task you can use a dispatcher such as the [`AsyncDispatcher`](crate::tasks::AsyncDispatcher) or [`ComputeDispatcher`](crate::tasks::ComputeDispatcher)
///
pub struct Task<T: Send + 'static> {
    pub(crate) join_handle: Option<JoinHandle<T>>,
}

impl<T: Send + 'static> Task<T> {
    /// Spawns a new, dead task
    pub fn new() -> Self {
        Self { join_handle: None }
    }

    /// Checks if the task is alive.
    pub fn is_alive(&self) -> bool {
        self.join_handle.is_some()
    }

    /// Polls the task status, returning `Some(T)` if it is completed and `None` if the task is still in progress or the task is dead.
    pub fn poll(&mut self) -> Option<T> {
        let output = match &mut self.join_handle {
            Some(join_handle) => future::block_on(async {
                future::poll_once(join_handle).await.map(|res| {
                    res.into_diagnostic()
                        .wrap_err("Failed to complete task")
                        .unwrap()
                })
            }),
            None => None,
        };

        // automatically kill the task so we don't poll a resolved future
        if output.is_some() {
            self.kill();
        }

        output
    }

    /// Kills the task, aborting the execution of anything that might be running.
    fn kill(&mut self) {
        if let Some(handle) = &self.join_handle {
            handle.abort();
        };

        self.join_handle = None;
    }
}

impl<T: Send + 'static> Default for Task<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// Provides a convenience method for adding corresponding tasks and resources to an app.
pub trait TaskResource {
    /// Consumes the [`Resource<T>`] and adds it, along with a dead [`Task<T>`] to the app storage.
    ///
    /// ```ignore
    /// fn main() {
    ///    let app = App::new();
    ///
    ///    app.add_task_resource(resource);
    ///    // Is equivalent to:
    ///    app.add_resource(Resource::<Task<T>>::default())?
    ///       .add_resource(resource);
    /// }
    /// ```
    fn add_task_resource<T: Send + Sync + 'static>(self, resource: Resource<T>) -> Result<Self>
    where
        Self: Sized;
}

impl TaskResource for App {
    fn add_task_resource<T: Send + Sync + 'static>(self, resource: Resource<T>) -> Result<Self>
    where
        Self: Sized,
    {
        self.add_resource(Resource::<Task<T>>::default())?
            .add_resource(resource)
    }
}
