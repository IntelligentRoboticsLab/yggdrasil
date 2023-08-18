use futures_lite::future;
use miette::Result;
use tokio::task::JoinHandle;

use tyr_internal::{App, Module, Resource};

use crate::tasks::{asynchronous::AsyncModule, compute::ComputeModule};

pub struct Task<T: Send + 'static> {
    pub(crate) join_handle: Option<JoinHandle<T>>,
}

impl<T: Send + 'static> Task<T> {
    pub fn new_dead() -> Self {
        Self { join_handle: None }
    }

    pub fn is_alive(&self) -> bool {
        self.join_handle.is_some()
    }

    pub fn poll(&mut self) -> Option<T> {
        let output = match &mut self.join_handle {
            Some(join_handle) => future::block_on(async {
                future::poll_once(join_handle)
                    .await
                    .map(|res| res.expect("Failed to join async task handle"))
            }),
            None => None,
        };

        // automatically kill the task so we don't poll a resolved future
        if output.is_some() {
            self.kill();
        }

        output
    }

    fn kill(&mut self) {
        if let Some(handle) = &self.join_handle {
            handle.abort();
        };

        self.join_handle = None;
    }
}

impl<T: Send + 'static> Default for Task<T> {
    fn default() -> Self {
        Self::new_dead()
    }
}

// TaskResource shouldn't be implementable for other types
mod sealed {
    use tyr_internal::App;

    pub trait Sealed {}
    impl Sealed for App {}
}

pub trait TaskResource: sealed::Sealed {
    /// Consumes the [`Resource<T>`] and adds it, along with a dead [`Task<T>`] to the app storage.
    fn add_task_resource<T: Send + Sync + 'static>(self, resource: Resource<T>) -> Result<Self>
    where
        Self: Sized;
}

impl TaskResource for App {
    fn add_task_resource<T: Send + Sync + 'static>(self, resource: Resource<T>) -> Result<Self>
    where
        Self: Sized,
    {
        self.add_resource(Resource::new(Task::<T>::default()))?
            .add_resource(resource)
    }
}

pub struct TaskModule;

impl Module for TaskModule {
    fn initialize(self, app: App) -> Result<App> {
        app.add_module(AsyncModule)?.add_module(ComputeModule)
    }
}
