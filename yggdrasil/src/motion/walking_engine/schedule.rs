use bevy::{ecs::schedule::InternedSystemSet, prelude::*};

use crate::prelude::*;
use crate::{behavior::engine::BehaviorState, kinematics::Kinematics};

use super::{config::WalkingEngineConfig, step::PlannedStep, step_context::StepContext};

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WalkingEngineSet {
    /// this runs right after we obtain new sensor data, but before running any logic
    Prepare,
    /// this runs right before generating the gait, in order to plan any required steps.
    PlanStep,
    /// this runs as the "main" loop of the walking engine, and generates the foot positions
    /// based on the current gait and state
    GenerateGait,
    /// this runs after the foot positions for the current gait have been computed and
    /// updates any balancing systems required
    Balance,
    /// this takes the target foot positions and the balance adjustments required and turns it into
    /// motion commands.
    Finalize,
}

impl WalkingEngineSet {
    /// The order of the walking engine system sets.
    fn order() -> impl IntoScheduleConfigs<InternedSystemSet, ()> {
        (
            Self::Prepare,
            Self::PlanStep,
            Self::GenerateGait,
            Self::Balance,
            Self::Finalize,
        )
            .chain()
    }
}

#[derive(States, Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Gait {
    #[default]
    Sitting,
    Standing,
    Starting,
    Walking,
    Stopping,
    Kicking,
}

/// Plugin that sets up the system sets that define the walking engine.
pub(super) struct WalkingEngineSchedulePlugin;

impl Plugin for WalkingEngineSchedulePlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<Gait>();
        app.configure_sets(PostStartup, WalkingEngineSet::order())
            .configure_sets(Sensor, WalkingEngineSet::order())
            .configure_sets(Update, WalkingEngineSet::order())
            .configure_sets(PreUpdate, WalkingEngineSet::order())
            .configure_sets(PostUpdate, WalkingEngineSet::order())
            .configure_sets(OnEnter(Gait::Sitting), WalkingEngineSet::order())
            .configure_sets(OnEnter(Gait::Standing), WalkingEngineSet::order())
            .configure_sets(OnEnter(Gait::Walking), WalkingEngineSet::order());

        app.add_systems(
            PostStartup,
            setup_motion_state.in_set(WalkingEngineSet::Finalize),
        );

        // Dirty hack: Reset the entire step context after standing up.
        app.add_systems(
            OnExit(BehaviorState::Standup),
            setup_motion_state.in_set(WalkingEngineSet::Finalize),
        );
    }
}

/// System that sets the initial [`MotionState`] depending on the initial hip height.
fn setup_motion_state(
    mut commands: Commands,
    config: Res<WalkingEngineConfig>,
    kinematics: Res<Kinematics>,
) {
    let hip_height = kinematics.left_hip_height();
    let planned = PlannedStep::default_from_kinematics(&kinematics, config.torso_offset);

    if hip_height >= config.hip_height.max_sitting_hip_height {
        commands.insert_resource(StepContext::init(Gait::Standing, planned));
    } else {
        commands.insert_resource(StepContext::init(Gait::Sitting, planned));
    }
}
