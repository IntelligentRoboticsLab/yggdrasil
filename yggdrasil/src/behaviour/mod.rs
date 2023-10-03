use miette::Result;
use tyr::prelude::*;

pub struct BehaviourModule;

impl Module for BehaviourModule {
    // Initializes the behaviour module.
    fn initialize(self, app: App) -> Result<App> {
        Ok(app
            .add_resource(Resource::new(GameState {}))?
            .add_startup_system(init_func)?
            .add_system(execution_func))
    }
}

// Added as resource in the initialize function of the BehaviourModule.
struct GameState {}

// Added as resource in the startup system.
struct RobotState {}

// Added as resource in the startup system.
struct CurrentAction {}

/// Called on startup
fn init_func(storage: &mut Storage) -> Result<()> {
    let robot_state = RobotState {};
    let current_action = CurrentAction {};
    storage.add_resource(Resource::new(robot_state))?;
    storage.add_resource(Resource::new(current_action))?;
    Ok(())
}

/// Called every iteration
#[system]
fn execution_func() -> Result<()> {
    println!("doing something");
    // Get relevant information (Gamestate, current state, positions of enemies)

    // Make struct for main decion state

    // Call decision state to change the state

    // Based on robotstate, change currentActions
        // make struct for robotstate
        // call action-deciders for the current robotstate
    Ok(())
}
