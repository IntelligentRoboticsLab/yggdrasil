use std::{any::type_name, future::Future};

use miette::{miette, IntoDiagnostic, Result};
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

    // Spawns the future on the async runtime and sets the task to alive status
    pub fn dispatch<F: Future + Send + 'static>(
        &self,
        task: &mut Task<F::Output>,
        future: F,
    ) -> Result<()>
    where
        F::Output: Send,
    {
        if task.is_alive() {
            // TODO: proper error types
            return Err(miette!(
                "Trying to dispatch task `{}` which is already alive!",
                type_name::<Task<F::Output>>()
            ));
        }

        let join_handle = Some(self.runtime_handle.spawn(future));

        *task = Task { join_handle };

        Ok(())
    }
}

pub fn initialize_runtime(storage: &mut Storage) -> Result<()> {
    let runtime = runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .into_diagnostic()?;

    let dispatcher = AsyncDispatcher::new(&runtime);

    storage.add_resource(Resource::new(runtime))?;
    storage.add_resource(Resource::new(dispatcher))?;

    Ok(())
}
