use std::time::Duration;

use crate::kinematics::Kinematics;

use super::{
    config::WalkingEngineConfig,
    feet::FootPositions,
    scheduling::{Gait, MotionSet},
    step::{PlannedStep, Step},
    FootSwitchedEvent, Side, TORSO_OFFSET,
};
use bevy::prelude::*;
use nalgebra::Vector2;

const MAX_ACCELERATION: Step = Step {
    forward: 0.01,
    left: 0.01,
    turn: 0.3,
};

pub(super) struct StepManagerPlugin;

impl Plugin for StepManagerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, sync_gait_request.in_set(MotionSet::StepPlanning));
        app.add_systems(PostUpdate, plan_step.in_set(MotionSet::StepPlanning));
    }
}

#[derive(Resource, Debug)]
pub struct StepManager {
    requested_gait: Gait,
    requested_step: Step,
    last_step: PlannedStep,
    pub planned_step: PlannedStep,
}

impl StepManager {
    pub fn new(gait: Gait, last_step: PlannedStep) -> Self {
        Self {
            requested_gait: Gait::Standing,
            requested_step: Step::default(),
            last_step: last_step.clone(),
            planned_step: last_step.clone(),
        }
    }

    pub fn request_sit(&mut self) {
        self.requested_gait = Gait::Sitting;
        self.finish_step();
    }

    pub fn request_stand(&mut self) {
        self.requested_gait = Gait::Standing;
        self.finish_step();
    }

    pub fn request_walk(&mut self, step: Step) {
        self.requested_gait = Gait::Walking;
        self.requested_step = step;
    }

    pub fn finish_step(&mut self) {
        self.last_step = self.planned_step;
    }

    pub fn plan_next_step(&mut self, start: FootPositions, config: &WalkingEngineConfig) {
        // clamp acceleration
        let delta_step =
            (self.requested_step - self.last_step.step).clamp(-MAX_ACCELERATION, MAX_ACCELERATION);

        // TODO(gijsd): do we want to assume this each time?
        let next_swing_foot = self.last_step.swing_foot.opposite();
        let next_step = (self.last_step.step + delta_step).clamp_anatomic(next_swing_foot, 0.1);

        info!(?delta_step, ?next_step, "delta step");

        let target = FootPositions::from_target(next_swing_foot, &next_step);
        let max_foot_lift = config.base_foot_lift
            + travel_weighting(
                start.swing_travel(next_swing_foot, &target).abs(),
                start.turn_amount(next_swing_foot, &target),
                config.foot_lift_modifier,
            );

        self.planned_step = PlannedStep {
            step: next_step,
            duration: Duration::from_millis(250),
            start,
            target,
            swing_foot_height: max_foot_lift,
            swing_foot: next_swing_foot,
        }
    }
}

pub(super) fn sync_gait_request(
    current: Res<State<Gait>>,
    mut next: ResMut<NextState<Gait>>,
    step_manager: Res<StepManager>,
) {
    if *current == step_manager.requested_gait {
        return;
    }

    info!(
        "switching requested gait to {:?}",
        step_manager.requested_gait
    );
    next.set(step_manager.requested_gait);
}

fn plan_step(
    mut event: EventReader<FootSwitchedEvent>,
    mut step_manager: ResMut<StepManager>,
    kinematics: Res<Kinematics>,
    config: Res<WalkingEngineConfig>,
) {
    for e in event.read() {
        let start = FootPositions::from_kinematics(e.0, &kinematics, TORSO_OFFSET);
        step_manager.finish_step();
        step_manager.plan_next_step(start, &config);
        break;
    }
}

fn travel_weighting(translation_travel: Vector2<f32>, turn_travel: f32, factors: Step) -> f32 {
    let translational = nalgebra::vector![
        factors.forward * translation_travel.x,
        factors.left * translation_travel.y,
    ]
    .norm();
    let rotational = factors.turn * turn_travel;
    translational + rotational
}
