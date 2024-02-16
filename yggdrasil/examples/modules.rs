//! An example that shows how initialization of an app can be compartmentalized using modules

use miette::Result;
use tyr::prelude::*;

struct FooModule;

#[system]
fn bar_system(counter: &mut i32) -> Result<()> {
    *counter += 1;
    Ok(())
}

impl Module for FooModule {
    fn initialize(self, app: App) -> Result<App> {
        Ok(app
            .add_resource(Resource::<i32>::new(32))?
            .add_system(bar_system))
    }
}

fn main() -> Result<()> {
    App::new().add_module(FooModule)?.run()
}
