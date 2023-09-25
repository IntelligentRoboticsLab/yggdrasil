use std::{collections::HashMap, hash::Hash, task::Poll};

use futures::{executor, poll};
use miette::{IntoDiagnostic, Result, WrapErr};
use tokio::task::JoinHandle;

use tyr_internal::{App, Resource, Storage};

// TODO: look at exports again
// TODO: get rid of some pub/pub(crates)
// TODO: look at all docs!!!
use crate::{asynchronous::AsyncTask, compute::ComputeTask, AsyncDispatcher, ComputeDispatcher};

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
pub struct Task<T: Send, D> {
    pub(crate) inner: Option<RawTask<T>>,
    pub(crate) dispatcher: D,
}

impl<T: Send + 'static, D> Task<T, D> {
    /// Spawns a new, dead task
    pub fn dead(dispatcher: D) -> Self {
        Self {
            inner: None,
            dispatcher,
        }
    }

    /// Checks if the task is alive.
    pub fn is_alive(&self) -> bool {
        self.inner.is_some()
    }

    pub fn poll(&mut self) -> Option<T> {
        let Some(task) = &mut self.inner else {
            return None;
        };

        match task.poll() {
            Some(output) => {
                self.inner = None;
                Some(output)
            }
            None => None,
        }
    }
}

pub(crate) struct RawTask<T: Send> {
    pub join_handle: JoinHandle<T>,
}

impl<T: Send> RawTask<T> {
    pub fn is_finished(&self) -> bool {
        self.join_handle.is_finished()
    }

    /// Polls the task status, returning `Some(T)` if it is completed and `None` if the task is still in progress or the task is dead.
    pub fn poll(&mut self) -> Option<T> {
        executor::block_on(async {
            match poll!(&mut self.join_handle) {
                Poll::Ready(res) => Some(
                    res.into_diagnostic()
                        .wrap_err("Failed to complete task")
                        .unwrap(),
                ),
                Poll::Pending => None,
            }
        })
    }
}

pub struct TaskMap<K: Hash + Eq + PartialEq, V: Send + 'static, D> {
    pub(crate) map: HashMap<K, RawTask<V>>,
    pub(crate) dispatcher: D,
}

impl<K: Hash + Eq + PartialEq, V: Send + 'static, D> TaskMap<K, V, D> {
    /// Spawns a new, dead [`TaskMap`]
    pub fn dead(dispatcher: D) -> Self {
        Self {
            map: HashMap::default(),
            dispatcher,
        }
    }

    /// Checks if task with key `K` is alive.
    pub fn is_alive(&self, key: &K) -> bool {
        self.map.contains_key(key)
    }

    /// Polls the task status, returning a T for every finished task.
    pub fn poll(&mut self) -> Vec<V> {
        // get all finished tasks
        let finished = self
            .map
            .iter_mut()
            .filter_map(|(_, raw)| raw.poll())
            .collect();

        // remove the unfinished tasks
        self.map.retain(|_, raw| !raw.is_finished());

        finished
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
    fn add_async_task<T: Send + Sync + 'static>(self) -> Result<Self>
    where
        Self: Sized;

    fn add_compute_task<T: Send + Sync + 'static>(self) -> Result<Self>
    where
        Self: Sized;
}

impl TaskResource for App {
    fn add_async_task<T: Send + Sync + 'static>(self) -> Result<Self>
    where
        Self: Sized,
    {
        fn add<T: Send + Sync + 'static>(s: &mut Storage) -> Result<()> {
            let dispatcher = s.map_resource_ref(|ad: &AsyncDispatcher| ad.clone());

            s.add_resource(Resource::new(AsyncTask::<T> {
                inner: None,
                dispatcher,
            }))?;

            Ok(())
        }

        self.add_startup_system(add::<T>)
    }

    fn add_compute_task<T: Send + Sync + 'static>(self) -> Result<Self>
    where
        Self: Sized,
    {
        fn add<T: Send + Sync + 'static>(s: &mut Storage) -> Result<()> {
            let dispatcher = s.map_resource_ref(|cd: &ComputeDispatcher| cd.clone());

            s.add_resource(Resource::new(ComputeTask::<T> {
                inner: None,
                dispatcher,
            }))?;

            Ok(())
        }

        self.add_startup_system(add::<T>)
    }
}
