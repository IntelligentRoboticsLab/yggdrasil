use std::time::Duration;

use super::{
    scheduling::{Gait, MotionSet},
    step::{PlannedStep, Step},
};
use bevy::prelude::*;

const MAX_ACCELERATION: Step = Step {
    forward: 0.01,
    left: 0.01,
    turn: 0.3,
};

pub(super) struct StepManagerPlugin;

impl Plugin for StepManagerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, sync_gait_request.in_set(MotionSet::StepPlanning));
    }
}

#[derive(Resource, Debug)]
pub struct StepManager {
    requested_gait: Gait,
    requested_step: Step,
    last_step: Step,
    pub planned_step: PlannedStep,
}

impl StepManager {
    pub fn request_sit(&mut self) {
        self.requested_gait = Gait::Sitting;
    }

    pub fn request_walk(&mut self, step: Step) {
        self.requested_gait = Gait::Walking;
        self.requested_step = step;
    }

    pub fn plan_next_step(&mut self) {
        // clamp acceleration
        let delta_step =
            (self.requested_step - self.last_step).clamp(-MAX_ACCELERATION, MAX_ACCELERATION);
        let next_step = self.requested_step + delta_step;

        self.planned_step = PlannedStep {
            forward: delta_step.forward,
            left: delta_step.left,
            turn: delta_step.turn,
            duration: Duration::from_millis(250),
            swing_foot_height: ,
            swing_foot: todo!(),
        }
    }
}

fn sync_gait_request(
    current: Res<State<Gait>>,
    mut next: ResMut<NextState<Gait>>,
    step_manager: Res<StepManager>,
) {
    if *current == step_manager.requested_gait {
        return;
    }

    next.set(step_manager.requested_gait);
}
