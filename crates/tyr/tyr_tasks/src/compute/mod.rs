mod threadpool;

pub use threadpool::ComputeDispatcher;

use miette::Result;
use tyr_internal::{App, Module};

pub struct ComputeModule;

impl Module for ComputeModule {
    fn initialize(self, app: App) -> Result<App> {
        app.add_startup_system(threadpool::initialize_threadpool)
    }
}
