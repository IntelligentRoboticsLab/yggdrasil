use std::{
    future::Future,
    panic::{catch_unwind, resume_unwind, AssertUnwindSafe},
    pin::Pin,
    task::{Context, Poll},
    thread,
};

use miette::{miette, IntoDiagnostic, Result};
use rayon::{ThreadPool, ThreadPoolBuilder};
use tokio::sync::oneshot::{self, Receiver};

use tyr_internal::{Resource, Storage};

use crate::{asynchronous::AsyncDispatcher, task::Task};

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
                .expect("Channel is closed")
                .unwrap_or_else(|e| resume_unwind(e))
        })
    }
}

pub struct ComputeDispatcher {
    thread_pool: ThreadPool,
    async_dispatcher: AsyncDispatcher,
}

#[allow(clippy::new_without_default)]
impl ComputeDispatcher {
    pub fn new(thread_pool: ThreadPool, async_dispatcher: AsyncDispatcher) -> Self {
        Self {
            thread_pool,
            async_dispatcher,
        }
    }

    pub fn dispatch<F, T>(&self, task: &mut Task<T>, func: F) -> Result<()>
    where
        F: FnOnce() -> T + Send + 'static,
        T: Send + 'static,
    {
        let (tx, rx) = oneshot::channel();

        self.thread_pool.spawn(move || {
            let _result = tx.send(catch_unwind(AssertUnwindSafe(func)));
        });

        let compute_join_handle = ComputeJoinHandle { rx };

        self.async_dispatcher.dispatch(task, compute_join_handle)
    }
}

pub fn initialize_threadpool(storage: &mut Storage) -> Result<()> {
    let thread_pool = ThreadPoolBuilder::new()
        .num_threads(2)
        .build()
        .into_diagnostic()?;

    let dispatcher = {
        let guard = storage
            .get::<AsyncDispatcher>()
            // TODO: proper error types
            .ok_or(miette!("No `AsyncDispatcher` found. `ComputeDispatcher` relies on the `AsyncDispatcher`. Did you import the AsyncModule?",
        ))?
        .read().unwrap();

        let ad: &AsyncDispatcher = guard.downcast_ref().unwrap();

        ComputeDispatcher::new(thread_pool, ad.clone())
    };

    storage.add_resource(Resource::new(dispatcher))?;

    Ok(())
}
