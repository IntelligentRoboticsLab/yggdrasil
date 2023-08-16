mod resource;
mod runtime;

pub use resource::AsyncResource;
pub use runtime::{AsyncDispatcher, AsyncTask};

use miette::Result;
use tyr::prelude::*;

pub struct AsyncModule;

impl Module for AsyncModule {
    fn initialize(self, app: App) -> Result<App> {
        app.add_startup_system(runtime::initialize_runtime)
    }
}
