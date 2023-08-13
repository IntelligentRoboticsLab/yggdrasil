use std::future::Future;

use miette::Result;
use tokio::{
    runtime::{self, Runtime},
    task::JoinHandle,
};

use tyr::prelude::*;

pub struct Task<T: Send + 'static> {
    pub(super) join_handle: JoinHandle<T>,
}

pub struct AsyncDispatcher {
    runtime: Runtime,
}

#[allow(clippy::new_without_default)]
impl AsyncDispatcher {
    pub fn new() -> Self {
        Self {
            runtime: runtime::Builder::new_multi_thread()
                .worker_threads(1)
                .enable_all()
                .build()
                .unwrap(),
        }
    }

    pub fn spawn<F: Future + Send + 'static>(&self, future: F) -> Task<F::Output>
    where
        F::Output: Send,
    {
        let join_handle = self.runtime.spawn(future);

        Task { join_handle }
    }
}

pub fn initialize_runtime(storage: &mut Storage) -> Result<()> {
    let dispatcher = AsyncDispatcher::new();

    storage.add_resource(Resource::new(dispatcher))?;

    Ok(())
}
