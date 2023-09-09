use std::{pin::pin, task::Poll};

use futures::{executor, poll};
use miette::{IntoDiagnostic, Result, WrapErr};
use tokio::task::{JoinHandle, JoinSet};

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
/// To activate a task you can use a dispatcher such as the [`AsyncDispatcher`](crate::AsyncDispatcher) or [`ComputeDispatcher`](crate::ComputeDispatcher)
pub struct Task<T: Send> {
    pub(crate) join_handle: Option<JoinHandle<T>>,
}

impl<T: Send + 'static> Task<T> {
    /// Spawns a new, dead task
    pub fn new() -> Self {
        Self::default()
    }

    /// Checks if the task is alive.
    pub fn is_alive(&self) -> bool {
        self.join_handle.is_some()
    }

    /// Polls the task status, returning `Some(T)` if it is completed and `None` if the task is still in progress or the task is dead.
    pub fn poll(&mut self) -> Option<T> {
        let output = match &mut self.join_handle {
            Some(join_handle) => executor::block_on(async {
                match poll!(join_handle) {
                    Poll::Ready(res) => Some(
                        res.into_diagnostic()
                            .wrap_err("Failed to complete task")
                            .unwrap(),
                    ),
                    Poll::Pending => None,
                }
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
        Self { join_handle: None }
    }
}

pub struct TaskSet<T: Send + 'static + Unpin> {
    pub(crate) join_set: JoinSet<T>,
}

impl<T: Send + 'static + Unpin> TaskSet<T> {
    /// Spawns a new, dead task set
    pub fn new() -> Self {
        Self::default()
    }

    /// Checks if any task is alive.
    pub fn is_alive(&self) -> bool {
        !self.join_set.is_empty()
    }

    /// Polls the task status, returning `Some(T)` if it is completed and `None` if the task is still in progress or the task is dead.
    pub fn poll_next(&mut self) -> Option<T> {
        if !self.is_alive() {
            return None;
        }

        executor::block_on(async {
            let future = self.join_set.join_next();
            match poll!(pin!(future)) {
                Poll::Ready(Some(res)) => Some(
                    res.into_diagnostic()
                        .wrap_err("Failed to complete task")
                        .unwrap(),
                ),
                // This should never occur as we check if there are any remaining tasks before this
                Poll::Ready(None) => unreachable!(),
                Poll::Pending => None,
            }
        })
    }

    /// Calls [`Self::poll_next`] for the amount of tasks in the set
    pub fn poll_all(&mut self) -> Vec<T> {
        (0..self.join_set.len())
            .flat_map(|_| self.poll_next())
            .collect()
    }
}

impl<T: Send + 'static + Unpin> Default for TaskSet<T> {
    fn default() -> Self {
        Self {
            join_set: JoinSet::new(),
        }
    }
}

/// Provides a convenience method for adding corresponding tasks and resources to an app.
pub trait TaskResource {
    /// Consumes the [`Resource<T>`] and adds it, along with a dead [`Task<T>`] to the app storage.
    ///
    /// ```
    /// use tyr::{prelude::*, tasks::{TaskResource, Task}};
    /// use miette::Result;
    ///
    /// fn main() -> Result<()> {
    ///     let app = App::new();
    ///
    ///     app.add_task_resource(Resource::new(1_i32))?;
    ///
    ///     // Is equivalent to:
    ///
    ///     let app2 = App::new()
    ///         .add_resource(Resource::<Task<i32>>::default())?
    ///         .add_resource(Resource::new(1_i32))?;
    ///
    ///    Ok(())
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
