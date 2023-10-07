use miette::Result;
use tyr::prelude::*;

mod robot_primary_state;
pub use robot_primary_state::RobotPrimaryState;
use robot_primary_state::RobotPrimaryStateModule;

pub struct BehaviourModule;

impl Module for BehaviourModule {
    fn initialize(self, app: App) -> Result<App> {
        Ok(app
            .add_module(RobotPrimaryStateModule)?
            .add_startup_system(initialize_states)?)
    }
}

struct CurrentAction {}

fn initialize_states(storage: &mut Storage) -> Result<()> {
    storage.add_resource(Resource::new(CurrentAction {}))?;

    Ok(())
}
