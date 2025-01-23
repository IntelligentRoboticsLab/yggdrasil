use bevy::prelude::*;

use crate::kinematics::Kinematics;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MotionSet {
    StepPlanning,
    GaitGeneration,
    Balancing,
    Finalize,
}

#[derive(States, Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MotionState {
    #[default]
    Sitting,
    Standing,
    Walking,
}

/// Plugin that sets up the system sets that define the motion engine.
pub(super) struct MotionSchedulePlugin;

impl Plugin for MotionSchedulePlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<MotionState>();
        app.add_systems(PostStartup, setup_motion_state);
        app.configure_sets(
            Update,
            (
                MotionSet::StepPlanning,
                MotionSet::GaitGeneration,
                MotionSet::Balancing,
                MotionSet::Finalize,
            )
                .chain(),
        );
    }
}

/// System that sets the initial [`MotionState`] depending on the initial hip height.
fn setup_motion_state(mut state: ResMut<NextState<MotionState>>, kinematics: Res<Kinematics>) {
    let hip_height = kinematics.left_hip_height();
    if hip_height >= 0.1 {
        state.set(MotionState::Sitting);
    } else {
        state.set(MotionState::Standing);
    }
}
