mod runtime;

pub use runtime::AsyncDispatcher;

use miette::Result;
use tyr_internal::{App, Module};

pub struct AsyncModule;

impl Module for AsyncModule {
    fn initialize(self, app: App) -> Result<App> {
        app.add_startup_system(runtime::initialize_runtime)
    }
}
