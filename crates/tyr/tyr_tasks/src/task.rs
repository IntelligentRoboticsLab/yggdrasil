use futures_lite::future;
use tokio::task::JoinHandle;

pub struct Task<T: Send + 'static> {
    pub(crate) join_handle: Option<JoinHandle<T>>,
}

impl<T: Send + 'static> Task<T> {
    pub fn new_dead() -> Self {
        Self { join_handle: None }
    }
}

impl<T: Send + 'static> Default for Task<T> {
    fn default() -> Self {
        Self::new_dead()
    }
}

impl<T: Send + 'static> Event<T> for Task<T> {
    type Data = Task<T>;

    fn spawn(&mut self, data: Self::Data) {
        *self = data;
    }

    fn is_alive(&self) -> bool {
        self.join_handle.is_some()
    }

    fn poll(&mut self) -> Option<T> {
        let output = match &mut self.join_handle {
            Some(join_handle) => future::block_on(async {
                future::poll_once(join_handle)
                    .await
                    .map(|res| res.expect("Failed to join async task handle"))
            }),
            None => None,
        };

        // automatically kill the task so we don't poll a completed future
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

use miette::Result;
use tyr_internal::{App, Resource};

use crate::event::Event;

// AsyncResource shouldn't be implementable for other types
mod sealed {
    use tyr_internal::App;

    pub trait Sealed {}
    impl Sealed for App {}
}

pub trait TaskResource: sealed::Sealed {
    /// Consumes the [`Resource<T>`] and adds it, along with a dead [`AsyncTask<T>`] to the app storage.
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
