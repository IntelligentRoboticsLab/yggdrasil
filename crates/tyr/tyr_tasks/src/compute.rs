//! Task types specialized for computationally heavy tasks
//!

use std::{
    future::Future,
    hash::Hash,
    panic::{catch_unwind, resume_unwind, AssertUnwindSafe},
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
    thread,
};

use miette::{IntoDiagnostic, WrapErr};
use rayon::ThreadPool;
use tokio::{
    runtime::Handle,
    sync::oneshot::{self, Receiver},
};
use tracing::trace_span;

use crate::{
    asynchronous::AsyncDispatcher,
    task::{Dispatcher, RawTask, Task, TaskMap, TaskSet},
    Error,
};

pub(super) struct RayonThreadPool(Arc<ThreadPool>);

impl RayonThreadPool {
    pub(super) fn new(thread_pool: Arc<ThreadPool>) -> Self {
        Self(thread_pool)
    }

    fn spawn<OP: FnOnce() + Send + 'static>(&self, op: OP) {
        self.0.spawn(op);
    }
}

impl Clone for RayonThreadPool {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

#[derive(Debug)]
struct ComputeJoinHandle<T> {
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
///
/// Can be used to obtain a handle to the underlying Tokio runtime.
#[derive(Clone)]
pub struct ComputeDispatcher {
    thread_pool: RayonThreadPool,
    async_dispatcher: AsyncDispatcher,
}

impl ComputeDispatcher {
    pub(super) fn new(thread_pool: RayonThreadPool, async_dispatcher: AsyncDispatcher) -> Self {
        Self {
            thread_pool,
            async_dispatcher,
        }
    }

    fn spawn<T: Send + 'static, F: FnOnce() -> T + Send + 'static>(
        &self,
        func: F,
    ) -> ComputeJoinHandle<T> {
        let (tx, rx) = oneshot::channel();
        self.thread_pool.spawn(move || {
            let span = trace_span!("compute", name = std::any::type_name::<T>());
            let _enter = span.enter();
            // send the result of invoking the function back through the oneshot channel,
            // capturing any panics that might occur
            let _result = tx.send(catch_unwind(AssertUnwindSafe(func)));
        });

        ComputeJoinHandle { rx }
    }
}

impl Dispatcher for ComputeDispatcher {
    fn handle(&self) -> &Handle {
        self.async_dispatcher.handle()
    }
}

/// A [`Task`] variant specialized for compute tasks.
///
/// For more explanation, see [`Task`].
///
/// To spawn a task, use the [`Self::try_spawn`](Task#impl-Task<T,+ComputeDispatcher>) method.
pub type ComputeTask<T> = Task<T, ComputeDispatcher>;

impl<T: Send + 'static> ComputeTask<T> {
    /// Tries to spawn the function on the thread pool and sets the task to be active by tracking
    /// its progress.
    ///
    /// # Errors
    /// Returns an [`Error::AlreadyActive`] if the task is active already.
    pub fn try_spawn<F>(&mut self, func: F) -> crate::Result<()>
    where
        F: FnOnce() -> T + Send + 'static,
        T: Send + 'static,
    {
        if self.active() {
            return Err(Error::AlreadyActive);
        }

        let compute_join_handle = self.dispatcher.spawn(func);
        let join_handle = self.dispatcher.async_dispatcher.spawn(compute_join_handle);

        self.raw = Some(RawTask { join_handle });

        Ok(())
    }
}

/// A [`TaskMap`] variant specialized for compute tasks.
///
/// For more explanation, see [`TaskMap`].
///
/// To spawn a task, use the [`Self::try_spawn`](TaskMap#impl-TaskMap<K,+T,+ComputeDispatcher>) method.
pub type ComputeTaskMap<K, T> = TaskMap<K, T, ComputeDispatcher>;

impl<K: Hash + Eq + PartialEq, T: Send + 'static> ComputeTaskMap<K, T> {
    /// Tries to spawn the function on the thread pool and adds progress tracking at the
    /// value of the given key.
    ///
    /// # Errors
    /// Returns an [`Error::AlreadyActive`] if the task for the given key is active already.
    pub fn try_spawn<F>(&mut self, key: K, func: F) -> crate::Result<()>
    where
        F: FnOnce() -> T + Send + 'static,
        T: Send + 'static,
    {
        if self.active(&key) {
            return Err(Error::AlreadyActive);
        }

        let compute_join_handle = self.dispatcher.spawn(func);
        let join_handle = self.dispatcher.async_dispatcher.spawn(compute_join_handle);

        self.map.insert(key, RawTask { join_handle });

        Ok(())
    }
}

/// A [`TaskSet`] variant specialized for compute tasks.
///
/// For more explanation, see [`TaskSet`].
///
/// To spawn a task, use the [`Self::spawn`](TaskSet#impl-TaskSet<T,+ComputeDispatcher>) method.
pub type ComputeTaskSet<T> = TaskSet<T, ComputeDispatcher>;

impl<T: Send + 'static> ComputeTaskSet<T> {
    /// Spawns the function on the thread pool and adds progress tracking to the set
    pub fn spawn<F>(&mut self, func: F)
    where
        F: FnOnce() -> T + Send + 'static,
        T: Send + 'static,
    {
        let compute_join_handle = self.dispatcher.spawn(func);

        let _guard = self.dispatcher.async_dispatcher.handle().enter();
        self.set.spawn(compute_join_handle);
    }
}
