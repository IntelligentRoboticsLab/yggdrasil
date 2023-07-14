use color_eyre::Result;
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
    tracing_subscriber::fmt::init();
    color_eyre::install()?;

    App::new().add_module(FooModule)?.run()?;

    Ok(())
}
