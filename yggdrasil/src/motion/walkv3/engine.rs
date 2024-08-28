use nidhogg::{
    types::{FillExt, LegJoints},
    NaoState,
};

use crate::{
    core::debug::DebugContext,
    kinematics::{self, forward::left_hip_to_ground, FootOffset, RobotKinematics},
    nao::{manager::NaoManager, CycleTime},
    prelude::*,
    sensor::button::ChestButton,
};

use super::action::{Action, Sitting, Standing, UpdateContext, WalkAction, Walking};

pub struct WalkingEnginev3 {
    pub action: Action,
}

impl WalkingEnginev3 {
    /// Constructs an initial [`WalkState`] from a `hip_height`.
    ///
    /// This returns a new [`WalkState::Sitting`] using an estimated current `hip_height`.
    pub fn from_hip_height(hip_height: f32) -> Self {
        let action = if hip_height > 0.1 {
            Action::Stand(Standing::with_hip_height(hip_height))
        } else {
            Action::Sit(Sitting::with_hip_height(hip_height))
        };

        Self { action }
    }
}

#[startup_system]
pub fn init_walking_engine(storage: &mut Storage, nao_state: &NaoState) -> Result<()> {
    let kinematics = RobotKinematics::from(&nao_state.position);
    let current_hip_height = left_hip_to_ground(&kinematics);

    tracing::info!("Current hip height: {}", current_hip_height);

    storage.add_resource(Resource::new(WalkingEnginev3::from_hip_height(
        current_hip_height,
    )))
}

#[system]
pub fn run_walking_enginev3(
    engine: &mut WalkingEnginev3,
    nao: &mut NaoManager,
    kinematics: &RobotKinematics,
    chest: &ChestButton,
    cycle_time: &CycleTime,
    dbg: &DebugContext,
) -> Result<()> {
    let ctx = UpdateContext {
        kinematics: kinematics.clone(),
        delta_time: cycle_time.duration,
    };
    engine.action.update(&ctx);
    engine.action.apply(nao, dbg);

    if chest.state.is_tapped() {
        match &engine.action {
            Action::Stand(_) => {
                engine.action = Action::Walk(Walking::new());
            }
            Action::Sit(sitting) => {
                engine.action =
                    Action::Stand(Standing::with_hip_height(sitting.current_hip_height));
            }
            Action::Walk(_) => {
                Action::Stand(Standing::with_hip_height(0.18));
            }
        }
    }

    Ok(())
}
