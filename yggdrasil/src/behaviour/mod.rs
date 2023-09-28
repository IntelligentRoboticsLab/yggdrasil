use miette::Result;
use tyr::prelude::*;

pub struct BehaviourModule;

impl Module for BehaviourModule {
    // Initializes the behaviour module.
    fn initialize(self, app: App) -> Result<App> {
        Ok(app
            .add_resource(Resource::new(WeCanStoreSomethingHere {}))?
            .add_startup_system(init_func)?
            .add_system(execution_func))
    }
}

// Added as resource in the initialize function of the BehaviourModule.
struct WeCanStoreSomethingHere {}

// Added as resource in the startup system.
struct WeCanStoreSomethingElseHere {}

/// Called on startup
fn init_func(storage: &mut Storage) -> Result<()> {
    let some_storage = WeCanStoreSomethingElseHere {};
    storage.add_resource(Resource::new(some_storage))?;
    Ok(())
}

/// Called every iteration
#[system]
fn execution_func() -> Result<()> {
    println!("doing something");
    Ok(())
}
