use std::future::Future;

use futures_lite::future;
use miette::Result;
use tokio::{
    runtime::{self, Runtime},
    task::JoinHandle,
};

use tyr::prelude::*;

use crate::event::Event;

pub struct AsyncTask<T: Send + 'static> {
    pub(super) join_handle: Option<JoinHandle<T>>,
}

impl<T: Send + 'static> AsyncTask<T> {
    pub fn new_dead() -> Self {
        Self { join_handle: None }
    }
}

impl<T: Send + 'static> Default for AsyncTask<T> {
    fn default() -> Self {
        Self::new_dead()
    }
}

impl<T: Send + 'static> Event<T> for AsyncTask<T> {
    type Data = AsyncTask<T>;

    fn spawn(&mut self, data: Self::Data) {
        *self = data;
    }

    fn is_alive(&self) -> bool {
        self.join_handle.is_some()
    }

    fn poll(&mut self) -> Option<T> {
        match &mut self.join_handle {
            Some(join_handle) => future::block_on(async {
                future::poll_once(join_handle)
                    .await
                    .map(|res| res.expect("Failed to join async task handle"))
            }),
            None => None,
        }
    }

    fn kill(&mut self) {
        self.join_handle = None;
    }
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

    pub fn dispatch<F: Future + Send + 'static>(&self, future: F) -> AsyncTask<F::Output>
    where
        F::Output: Send,
    {
        let join_handle = Some(self.runtime.spawn(future));

        AsyncTask { join_handle }
    }
}

pub fn initialize_runtime(storage: &mut Storage) -> Result<()> {
    let dispatcher = AsyncDispatcher::new();

    storage.add_resource(Resource::new(dispatcher))?;

    Ok(())
}
