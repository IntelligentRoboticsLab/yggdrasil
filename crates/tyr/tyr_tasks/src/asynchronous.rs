//! Task types specialized for asynchronous tasks (I/O, networking)
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

    pub(super) fn spawn_blocking<T: Send + 'static, F: FnOnce() -> T + Send + 'static>(
        &self,
        func: F,
    ) -> JoinHandle<T> {
        self.runtime_handle.spawn_blocking(func)
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

    /// Tries to spawn the *BLOCKING* future on the async runtime and sets the task to be active by tracking
    /// its progress.
    ///
    /// # Errors
    /// Returns an [`Error::AlreadyActive`] if the task is active already.
    pub fn try_spawn_blocking<F: FnOnce() -> T + Send + 'static>(
        &mut self,
        func: F,
    ) -> crate::Result<()> {
        if self.active() {
            return Err(Error::AlreadyActive);
        }

        let join_handle = self.dispatcher.spawn_blocking(func);

        self.raw = Some(RawTask { join_handle });

        Ok(())
    }

    /// Tries to the cancel the *RUNNING* task.
    ///
    /// # Errors
    /// Returns an [`Error::NotActive`] if the task is not active.
    pub fn try_cancel(&mut self) -> crate::Result<()> {
        self.raw.take().ok_or(Error::NotActive)?.join_handle.abort();
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
