pub mod runtime;

use futures_lite::future;
use miette::Result;
use runtime::Task;
use tyr::prelude::*;

pub struct AsyncModule;

impl Module for AsyncModule {
    fn initialize(self, app: App) -> Result<App> {
        app.add_startup_system(runtime::initialize_runtime)
    }
}

pub fn poll_task<T: Send + 'static>(task: &mut Task<T>) -> Option<T> {
    future::block_on(async {
        future::poll_once(&mut task.join_handle)
            .await
            .map(|res| res.expect("Failed to join async task handle"))
    })
}
