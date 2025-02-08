use std::time::Duration;

use bevy::prelude::*;

use crate::kinematics::Kinematics;

use super::{
    config::WalkingEngineConfig,
    feet::FootPositions,
    step::{PlannedStep, Step},
    step_manager::StepManager,
    Side, TORSO_OFFSET,
};

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MotionSet {
    StepPlanning,
    GaitGeneration,
    Balancing,
    Finalize,
}

impl MotionSet {
    /// The order of the motion system sets.
    fn order() -> impl IntoSystemSetConfigs {
        (
            MotionSet::StepPlanning,
            MotionSet::GaitGeneration,
            MotionSet::Balancing,
            MotionSet::Finalize,
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
}

/// Plugin that sets up the system sets that define the motion engine.
pub(super) struct MotionSchedulePlugin;

impl Plugin for MotionSchedulePlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<Gait>();
        app.configure_sets(PostStartup, MotionSet::order())
            .configure_sets(Update, MotionSet::order())
            .configure_sets(PostUpdate, MotionSet::order())
            .configure_sets(OnEnter(Gait::Sitting), MotionSet::order())
            .configure_sets(OnEnter(Gait::Standing), MotionSet::order())
            .configure_sets(OnEnter(Gait::Walking), MotionSet::order());

        app.add_systems(PostStartup, setup_motion_state.in_set(MotionSet::Finalize));
    }
}

/// System that sets the initial [`MotionState`] depending on the initial hip height.
fn setup_motion_state(
    mut commands: Commands,
    config: Res<WalkingEngineConfig>,
    kinematics: Res<Kinematics>,
) {
    let start = FootPositions::from_kinematics(Side::Left, &kinematics, TORSO_OFFSET);

    let hip_height = kinematics.left_hip_height();
    let planned = PlannedStep {
        step: Step::default(),
        start: start.clone(),
        target: start.clone(),
        duration: Duration::from_millis(250),
        swing_foot_height: 0.,
        swing_foot: Side::Left,
    };

    if hip_height >= config.max_sitting_hip_height {
        commands.insert_resource(StepManager::init(Gait::Standing, planned));
    } else {
        commands.insert_resource(StepManager::init(Gait::Sitting, planned));
    }
}
