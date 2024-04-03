//! Basic traits and types for managing tasks
//!
//! Tasks allow functions to execute over multiple system cycles.
//!
//! Tasks might be in either an active or inactive state.
//! - A task is active when the execution of code is being awaited or calculated.
//! - A task is inactive when there is nothing to be awaited or calculated.
//!
//! To get the result out of a task, you must check if it is completed using the [`Pollable::poll`] method.
//!
//! # Note 📝
//!
//! Tasks are identified by their return type, so multiple tasks returning the same type would interfere
//! (even returning 'nothing' returns `()` implicitly)! Therefore it is always recommended to create a unique
//! struct for the task output, even if the code executed doesn't need to return anything.
//!
//! ## Example
//! ```
//! // A struct without any data associated, but that uniquely identifies this task
//! struct Completed;
//!
//! fn calculation() -> Completed {
//!     // Do calculation...
//!     
//!     // Return an instance of the `Completed` struct
//!     Completed
//! }
//! ```  
//!

use std::{collections::HashMap, hash::Hash, pin::pin, task::Poll};

use futures::poll;
use miette::{IntoDiagnostic, Result, WrapErr};
use tokio::{
    runtime::Handle,
    task::{JoinHandle, JoinSet},
};

use tyr_internal::{self, App, Res, Resource, Storage};

/// A dispatcher manages tasks of a specific type (e.g. async/compute).
pub trait Dispatcher {
    /// Returns a raw tokio [`Handle`] to the underlying runtime.
    ///
    /// # Warning ⚠️
    /// If you are looking to run async functions, this is often not what you want.
    /// When using Tokio directly, it is easy to block the main thread or spawn futures repeatedly by accident.
    ///
    /// You should try one of the `AsyncTask*` types first.
    ///
    /// If you do think you need this, remember that **with great power comes great responsibility**.
    ///
    fn handle(&self) -> &Handle;
}

/// Methods for creating and polling tasks.
pub trait Pollable {
    /// The type of dispatcher this task is using.
    type Dispatcher: Dispatcher;
    /// The type of the value this task returns.
    type Output;

    /// Creates a new inactive task.
    fn new(dispatcher: Self::Dispatcher) -> Self;

    /// Checks the task progress and returns any new results.
    fn poll(&mut self) -> Self::Output;
}

pub(super) struct RawTask<T: Send> {
    pub join_handle: JoinHandle<T>,
}

impl<T: Send> RawTask<T> {
    /// Check if the task is finished and a value can be returned.
    pub fn is_finished(&self) -> bool {
        self.join_handle.is_finished()
    }

    /// Polls the task status, returning `Some(T)` if the task is finished and `None` if the task is still in progress or inactive.
    pub fn poll(&mut self, handle: &Handle) -> Option<T> {
        handle.block_on(async {
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

/// The simplest kind of task storage. It can await execution of one task at a time.
///
/// # Note 📝
/// You probably don't need to use this type directly.
/// Instead, use:
/// - [`AsyncTask<T>`](`crate::asynchronous::AsyncTask`)
/// - [`ComputeTask<T>`](`crate::compute::ComputeTask`)
pub struct Task<T: Send, D: Dispatcher> {
    pub(super) raw: Option<RawTask<T>>,
    pub(super) dispatcher: D,
}

impl<T: Send, D: Dispatcher> Task<T, D> {
    /// Checks if the task is active (something is being executed) or not.
    pub fn active(&self) -> bool {
        self.raw.is_some()
    }
}

impl<T: Send, D: Dispatcher> Pollable for Task<T, D> {
    type Dispatcher = D;
    type Output = Option<T>;

    fn new(dispatcher: Self::Dispatcher) -> Self {
        Self {
            raw: None,
            dispatcher,
        }
    }

    fn poll(&mut self) -> Self::Output {
        let task = &mut self.raw.as_mut()?;

        match task.poll(self.dispatcher.handle()) {
            Some(output) => {
                self.raw = None;
                Some(output)
            }
            None => None,
        }
    }
}

/// A key/value based task storage. Use it if you want to await execution of one task per key.
///
/// # Note 📝
/// You probably don't need to use this type directly.
/// Instead, use:
/// - [`AsyncTaskMap<T>`](`crate::asynchronous::AsyncTaskMap`)
/// - [`ComputeTaskMap<T>`](`crate::compute::ComputeTaskMap`)
pub struct TaskMap<K: Hash + Eq + PartialEq, V: Send + 'static, D: Dispatcher> {
    pub(super) map: HashMap<K, RawTask<V>>,
    pub(super) dispatcher: D,
}

impl<K: Hash + Eq + PartialEq, V: Send + 'static, D: Dispatcher> TaskMap<K, V, D> {
    /// Checks if the task associated with a key is active (something is being executed) or not.
    pub fn active(&self, key: &K) -> bool {
        self.map.contains_key(key)
    }
}

impl<K: Hash + Eq + PartialEq, V: Send + 'static, D: Dispatcher> Pollable for TaskMap<K, V, D> {
    type Dispatcher = D;
    type Output = Vec<V>;

    fn new(dispatcher: Self::Dispatcher) -> Self {
        Self {
            map: HashMap::default(),
            dispatcher,
        }
    }

    /// Polls the task status, returning a T for every finished task.
    fn poll(&mut self) -> Self::Output {
        // get all finished tasks
        let finished = self
            .map
            .iter_mut()
            .filter_map(|(_, raw)| raw.poll(self.dispatcher.handle()))
            .collect();

        // remove the unfinished tasks
        self.map.retain(|_, raw| !raw.is_finished());

        finished
    }
}

/// An unordered set of tasks. Allows for any amount of tasks to be stored, and they may finish in any order.
///
/// # Note 📝
/// You probably don't need to use this type directly.
/// Instead, use:
/// - [`AsyncTaskSet<T>`](`crate::asynchronous::AsyncTaskSet`)
/// - [`ComputeTaskSet<T>`](`crate::compute::ComputeTaskSet`)
pub struct TaskSet<T: Send + 'static, D: Dispatcher> {
    pub(super) set: JoinSet<T>,
    pub(super) dispatcher: D,
}

impl<T: Send + 'static, D: Dispatcher> Pollable for TaskSet<T, D> {
    type Dispatcher = D;
    type Output = Vec<T>;

    fn new(dispatcher: Self::Dispatcher) -> Self {
        Self {
            set: JoinSet::new(),
            dispatcher,
        }
    }

    /// Polls the task status, returning a T for every finished task.
    fn poll(&mut self) -> Self::Output {
        self.dispatcher.handle().block_on(async {
            let mut completed = vec![];

            // Poll once for every task in the set
            for _ in 0..self.set.len() {
                let poll_result = match poll!(pin!(self.set.join_next())) {
                    Poll::Ready(v) => v.transpose().unwrap(),
                    Poll::Pending => None,
                };

                // We might have polled an empty set
                if let Some(value) = poll_result {
                    completed.push(value);
                };
            }

            completed
        })
    }
}

/// Provides a convenience method for adding tasks to an app.
pub trait TaskResource {
    /// Adds a task to the app that gets initialized with its corresponding dispatcher.
    ///
    /// # Errors
    /// This function fails if the needed dispatcher doesn't already exist in storage,
    /// or if the task already exists in storage.
    fn add_task<T>(self) -> Result<Self>
    where
        Self: Sized,
        T: Pollable + Send + Sync + 'static,
        T::Dispatcher: Clone + Send + Sync;
}

impl TaskResource for App {
    fn add_task<T>(self) -> Result<Self>
    where
        Self: Sized,
        T: Pollable + Send + Sync + 'static,
        T::Dispatcher: Clone + Send + Sync,
    {
        fn _add_task<T: Pollable + Send + Sync + 'static>(
            storage: &mut Storage,
            dispatcher: Res<T::Dispatcher>,
        ) -> Result<()>
        where
            T::Dispatcher: Clone + Send + Sync,
        {
            storage.add_resource(Resource::new(T::new(dispatcher.clone())))?;

            Ok(())
        }

        self.add_startup_system(_add_task::<T>)
    }
}
