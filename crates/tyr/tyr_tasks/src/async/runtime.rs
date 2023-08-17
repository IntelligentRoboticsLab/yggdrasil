use std::future::Future;

use miette::Result;
use tokio::runtime::{self, Handle, Runtime};
use tyr_internal::{Resource, Storage};

use crate::task::Task;

#[derive(Clone)]
pub struct AsyncDispatcher {
    runtime_handle: Handle,
}

#[allow(clippy::new_without_default)]
impl AsyncDispatcher {
    pub fn new(runtime: &Runtime) -> Self {
        Self {
            runtime_handle: runtime.handle().clone(),
        }
    }

    pub fn dispatch<F: Future + Send + 'static>(&self, future: F) -> Task<F::Output>
    where
        F::Output: Send,
    {
        let join_handle = Some(self.runtime_handle.spawn(future));

        Task { join_handle }
    }
}

pub fn initialize_runtime(storage: &mut Storage) -> Result<()> {
    let runtime = runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .unwrap();

    let dispatcher = AsyncDispatcher::new(&runtime);

    storage.add_resource(Resource::new(runtime))?;
    storage.add_resource(Resource::new(dispatcher))?;

    Ok(())
}
