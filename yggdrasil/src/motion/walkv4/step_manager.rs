use std::time::Duration;

use crate::{
    core::debug::{
        debug_system::{DebugAppExt, SystemToggle},
        DebugContext,
    },
    kinematics::{
        prelude::{ROBOT_TO_LEFT_PELVIS, ROBOT_TO_RIGHT_PELVIS},
        Kinematics,
    },
    nao::Cycle,
    prelude::PreWrite,
};

use super::{
    config::WalkingEngineConfig,
    feet::FootPositions,
    schedule::{Gait, MotionSet, StepPlanning},
    step::{PlannedStep, Step},
    FootSwitchedEvent, TORSO_OFFSET,
};
use bevy::prelude::*;
use nalgebra::Vector2;
use rerun::external::glam::{Quat, Vec3};

const MAX_ACCELERATION: Step = Step {
    forward: 0.01,
    left: 0.01,
    turn: 0.3,
};

pub(super) struct StepManagerPlugin;

impl Plugin for StepManagerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostStartup, setup_step_visualizer);
        app.add_systems(
            StepPlanning,
            sync_gait_request.in_set(MotionSet::StepPlanning),
        );

        // TODO: Probably want a separate schedule for this!
        app.add_systems(
            PreWrite,
            plan_step
                .run_if(on_event::<FootSwitchedEvent>)
                .in_set(MotionSet::StepPlanning),
        );
        app.add_named_debug_systems(
            PreWrite,
            visualize_planned_step
                .after(plan_step)
                .run_if(on_event::<FootSwitchedEvent>)
                .in_set(MotionSet::StepPlanning),
            "Visualize planned step",
            SystemToggle::Enable,
        );
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
    #[must_use]
    pub fn init(gait: Gait, last_step: PlannedStep) -> Self {
        Self {
            requested_gait: gait,
            requested_step: Step::default(),
            last_step,
            planned_step: last_step,
        }
    }

    pub fn request_sit(&mut self) {
        self.requested_gait = Gait::Sitting;
        self.last_step = PlannedStep::default();
        self.requested_step = Step::default();
    }

    pub fn request_stand(&mut self) {
        self.requested_gait = Gait::Standing;
        self.last_step = PlannedStep::default();
        self.requested_step = Step::default();
    }

    pub fn request_walk(&mut self, step: Step) {
        match self.requested_gait {
            Gait::Sitting => error!(
                "Cannot request walk while sitting! Call StepManager::request_stand() first!"
            ),
            Gait::Standing => {
                // go to starting
                self.requested_gait = Gait::Starting;
                self.requested_step = step;
            }
            Gait::Starting | Gait::Walking => {
                // the robot is currently starting or walking already, so we just change the requested step
                self.requested_step = step;
            }
        }
    }

    pub fn finish_step(&mut self) {
        self.last_step = self.planned_step;
    }

    pub(super) fn finish_starting_step(&mut self, step: PlannedStep) {
        self.last_step = step;
        self.requested_gait = Gait::Walking;
    }

    pub fn plan_next_step(&mut self, start: FootPositions, config: &WalkingEngineConfig) {
        // clamp acceleration
        let delta_step =
            (self.requested_step - self.last_step.step).clamp(-MAX_ACCELERATION, MAX_ACCELERATION);

        // TODO(gijsd): do we want to assume this each time?
        let next_swing_foot = self.last_step.swing_foot.opposite();
        let next_step = (self.last_step.step + delta_step).clamp_anatomic(next_swing_foot, 0.1);

        let target = FootPositions::from_target(next_swing_foot, &next_step);
        let swing_travel = start.swing_travel(next_swing_foot, &target).abs();
        let turn_amount = start.turn_amount(next_swing_foot, &target);

        let foot_lift_modifier =
            travel_weighting(swing_travel, turn_amount, config.foot_lift_modifier);

        let step_duration_modifier = Duration::from_secs_f32(travel_weighting(
            swing_travel,
            turn_amount,
            config.step_duration_modifier,
        ));

        self.planned_step = PlannedStep {
            step: next_step,
            duration: config.base_step_duration + step_duration_modifier,
            start,
            target,
            swing_foot_height: config.base_foot_lift + foot_lift_modifier,
            swing_foot: next_swing_foot,
        }
    }
}

fn setup_step_visualizer(dbg: DebugContext) {
    dbg.log_static(
        "nao/planned_left_foot",
        &rerun::Asset3D::from_file("./assets/rerun/left_foot.glb")
            .expect("Failed to load left step model")
            .with_media_type(rerun::MediaType::glb()),
    );

    dbg.log_static(
        "nao/planned_right_foot",
        &rerun::Asset3D::from_file("./assets/rerun/right_foot.glb")
            .expect("Failed to load left step model")
            .with_media_type(rerun::MediaType::glb()),
    );
}

pub(super) fn sync_gait_request(
    mut commands: Commands,
    current: Res<State<Gait>>,
    step_manager: Res<StepManager>,
) {
    if *current == step_manager.requested_gait {
        return;
    }

    info!(
        "switching requested gait to {:?}",
        step_manager.requested_gait
    );
    commands.set_state(step_manager.requested_gait);
}

fn plan_step(
    mut event: EventReader<FootSwitchedEvent>,
    mut step_manager: ResMut<StepManager>,
    kinematics: Res<Kinematics>,
    config: Res<WalkingEngineConfig>,
) {
    let Some(event) = event.read().next() else {
        return;
    };

    let start = FootPositions::from_kinematics(event.0, &kinematics, TORSO_OFFSET);
    step_manager.finish_step();
    step_manager.plan_next_step(start, &config);
}

fn travel_weighting(swing_travel: Vector2<f32>, turn_amount: f32, weights: Step) -> f32 {
    let translational = nalgebra::vector![
        weights.forward * swing_travel.x,
        weights.left * swing_travel.y,
    ]
    .norm();
    let rotational = weights.turn * turn_amount;
    translational + rotational
}

fn visualize_planned_step(dbg: DebugContext, cycle: Res<Cycle>, step_manager: Res<StepManager>) {
    let planned = step_manager.planned_step;
    dbg.log_with_cycle(
        "nao/planned_left_foot",
        *cycle,
        &rerun::Transform3D::update_fields()
            .with_translation(Vec3::from(
                planned.target.left.translation.vector - ROBOT_TO_LEFT_PELVIS * 2.,
            ))
            .with_quaternion(Quat::from(planned.target.left.rotation)),
    );

    dbg.log_with_cycle(
        "nao/planned_right_foot",
        *cycle,
        &rerun::Transform3D::update_fields()
            .with_translation(Vec3::from(
                planned.target.right.translation.vector - ROBOT_TO_RIGHT_PELVIS * 2.,
            ))
            .with_quaternion(Quat::from(planned.target.right.rotation)),
    );
}
