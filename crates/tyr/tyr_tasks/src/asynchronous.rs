//! Task types specialized for asynchronous tasks (I/O, networking)
//!
//! # Example
//! ```
//! use tyr::prelude::*;
//! use miette::Result;
//!
//! struct Euros(i32);
//!
//! async fn do_launder() -> Euros {
//!     // Needs to connect to a secret bank across the globe...
//!     // This may take a while!
//!     // ...
//!     // We laundered 20 euros! Nice!
//!     Euros(20)
//! }
//!
//! // In this system we want to handle spawning our task.
//! #[system]
//! fn launder_money(task: &mut AsyncTask<Euros>) -> Result<()> {
//!     // We want to launder money only with one connection at a time,
//!     // as more connections would get suspicious.
//!
//!     // If you want to handle multiple tasks at any given time, take a look
//!     // at the `TaskMap` or `TaskSet`.
//!
//!     // Because tasks run can execute over multiple system cycles,
//!     // this system might run at a time the task is already active.
//!     // therefore `try_spawn` can match with two different variants
//!     match task.try_spawn(do_launder()) {
//!         // Option 1: The task was inactive, so we can successfully spawn the task
//!         // Also marks the task as active while we haven't received the result,
//!         // so we can't accidentally dispatch it twice.
//!         Ok(_) => Ok(()),
//!
//!         // Option 2: The task was already active, we don't need to do anything
//!         // else here, so we can just return from the function normally.
//!         Err(Error::AlreadyActive) => Ok(()),
//!     }
//! }
//!
//! // In this system we want to handle the completion of tasks
//! #[system]
//! fn handle_completion(
//!     task: &mut AsyncTask<Euros>,
//! ) -> Result<()> {
//!     // We check if the task has completed by polling it.
//!     // If there is Some(money), we can use it in the function,
//!     // if there is None, we return early
//!     let Some(money) = task.poll() else {
//!         // Task is not yet ready
//!         return Ok(());
//!     };
//!
//!     // Our task has returned something in this cycle!
//!     // We can now use `money` here!
//!
//!     Ok(())
//! }
//! ```
//!

use std::{future::Future, hash::Hash};

use tokio::{
    runtime::{Handle, Runtime},
    task::JoinHandle,
};

use crate::{
    task::{Dispatcher, RawTask, Task, TaskMap, TaskSet},
    Error,
};

pub(super) struct TokioRuntime(Runtime);

impl TokioRuntime {
    pub(super) fn new(runtime: Runtime) -> Self {
        Self(runtime)
    }

    /// Returns a raw tokio [`Handle`] to the underlying runtime
    pub fn handle(&self) -> &Handle {
        self.0.handle()
    }
}

/// Dispatcher that allows futures to run efficiently on a separate thread without blocking the main thread.
///
/// Can be used to obtain a handle to the underlying Tokio runtime.
#[derive(Clone)]
pub struct AsyncDispatcher {
    runtime_handle: Handle,
}

impl AsyncDispatcher {
    pub(super) fn new(runtime_handle: Handle) -> Self {
        Self { runtime_handle }
    }

    pub(super) fn spawn<T: Send + 'static, F: Future<Output = T> + Send + 'static>(
        &self,
        future: F,
    ) -> JoinHandle<T> {
        self.runtime_handle.spawn(future)
    }
}

impl Dispatcher for AsyncDispatcher {
    fn handle(&self) -> &Handle {
        &self.runtime_handle
    }
}

/// A [`Task`] variant specialized for asynchronous tasks.
///
/// For more explanation, see [`Task`].
///
/// To spawn a task, use the [`Self::try_spawn`](Task#impl-Task<T,+AsyncDispatcher>) method.
pub type AsyncTask<T> = Task<T, AsyncDispatcher>;

impl<T: Send + 'static> AsyncTask<T> {
    /// Tries to spawn the future on the async runtime and sets the task to be active by tracking
    /// its progress.
    ///
    /// # Errors
    /// Returns an [`Error::AlreadyActive`] if the task is active already.
    pub fn try_spawn<F: Future<Output = T> + Send + 'static>(
        &mut self,
        future: F,
    ) -> crate::Result<()> {
        if self.active() {
            return Err(Error::AlreadyActive);
        }

        let join_handle = self.dispatcher.spawn(future);

        self.raw = Some(RawTask { join_handle });

        Ok(())
    }
}

/// A [`TaskMap`] variant specialized for asynchronous tasks.
///
/// For more explanation, see [`TaskMap`].
///
/// To spawn a task, use the [`Self::try_spawn`](TaskMap#impl-TaskMap<K,+T,+AsyncDispatcher>) method.
pub type AsyncTaskMap<K, T> = TaskMap<K, T, AsyncDispatcher>;

impl<K: Hash + Eq + PartialEq, T: Send + 'static> AsyncTaskMap<K, T> {
    /// Tries to spawn the future on the async runtime and adds progress tracking at the
    /// value of the given key.
    ///
    /// # Errors
    /// Returns an [`Error::AlreadyActive`] if there is already an active task for the key
    pub fn try_spawn<F: Future<Output = T> + Send + 'static>(
        &mut self,
        key: K,
        future: F,
    ) -> crate::Result<()> {
        if self.active(&key) {
            return Err(Error::AlreadyActive);
        }

        let join_handle = self.dispatcher.spawn(future);

        self.map.insert(key, RawTask { join_handle });

        Ok(())
    }
}

/// A [`TaskSet`] variant specialized for asynchronous tasks.
///
/// For more explanation, see [`TaskSet`].
///
/// To spawn a task, use the [`Self::spawn`](TaskSet#impl-TaskSet<T,+AsyncDispatcher>) method.
pub type AsyncTaskSet<T> = TaskSet<T, AsyncDispatcher>;

impl<T: Send + 'static> AsyncTaskSet<T> {
    /// Spawns the future on the async runtime and adds progress tracking to the set
    pub fn spawn<F: Future<Output = T> + Send + 'static>(&mut self, future: F) {
        let _guard = self.dispatcher.runtime_handle.enter();
        self.set.spawn(future);
    }
}
