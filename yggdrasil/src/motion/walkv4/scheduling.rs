use bevy::prelude::*;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MotionSet {
    StepPlanning,
    GaitGeneration,
    Balancing,
    Finalize,
}

/// Plugin that sets up the system sets that define the motion engine.
pub(super) struct MotionSchedulePlugin;

impl Plugin for MotionSchedulePlugin {
    fn build(&self, app: &mut App) {
        app.configure_sets(
            Update,
            (
                MotionSet::StepPlanning,
                MotionSet::GaitGeneration,
                MotionSet::Balancing,
                MotionSet::Finalize,
            ),
        );
    }
}
