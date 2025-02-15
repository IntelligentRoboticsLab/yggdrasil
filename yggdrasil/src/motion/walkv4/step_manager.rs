use std::time::Duration;

use crate::{
    core::debug::{
        debug_system::{DebugAppExt, SystemToggle},
        DebugContext,
    },
    kinematics::Kinematics,
    nao::Cycle,
};

use super::{
    config::WalkingEngineConfig,
    feet::FootPositions,
    schedule::{Gait, WalkingEngineSet},
    step::{PlannedStep, Step},
    FootSwitchedEvent,
};
use bevy::prelude::*;
use nalgebra::Vector2;
use rerun::external::glam::{Quat, Vec3};

pub(super) struct StepManagerPlugin;

impl Plugin for StepManagerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostStartup, setup_step_visualizer);
        app.add_systems(
            PreUpdate,
            sync_gait_request.in_set(WalkingEngineSet::Prepare),
        );

        // TODO: Probably want a separate schedule for this!
        app.add_systems(
            PreUpdate,
            plan_step
                .run_if(on_event::<FootSwitchedEvent>)
                .in_set(WalkingEngineSet::PlanStep),
        );
        app.add_named_debug_systems(
            Update,
            visualize_planned_step
                .after(plan_step)
                .run_if(on_event::<FootSwitchedEvent>)
                .in_set(WalkingEngineSet::PlanStep),
            "Visualize planned step",
            SystemToggle::Enable,
        );
    }
}

#[derive(Resource, Debug)]
pub struct StepContext {
    requested_gait: Gait,
    requested_step: Step,
    last_step: PlannedStep,
    pub planned_step: PlannedStep,
}

impl StepContext {
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
        self.last_step = PlannedStep {
            swing_side: self.last_step.swing_side,
            ..Default::default()
        };
        self.requested_step = Step::default();
    }

    pub fn request_stand(&mut self) {
        self.requested_gait = Gait::Standing;
        self.last_step = PlannedStep {
            swing_side: self.last_step.swing_side,
            ..Default::default()
        };
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
        let delta_step = (self.requested_step - self.last_step.step)
            .clamp(-config.max_acceleration, config.max_acceleration);

        // TODO(gijsd): do we want to assume this each time?
        let next_swing_foot = self.last_step.swing_side.opposite();
        info!("next swing_foot: {:?}", next_swing_foot);
        let next_step = (self.last_step.step + delta_step)
            .clamp(-config.max_step_size, config.max_step_size)
            .clamp_anatomic(next_swing_foot, 0.1);

        let target = FootPositions::from_target(next_swing_foot, &next_step);
        let swing_translation = start.swing_translation(next_swing_foot, &target).abs();
        let turn_amount = start.turn_amount(next_swing_foot, &target);

        let foot_lift_modifier =
            translation_weight(swing_translation, turn_amount, config.foot_lift_modifier);

        let step_duration_modifier = Duration::from_secs_f32(translation_weight(
            swing_translation,
            turn_amount,
            config.step_duration_modifier,
        ));

        self.planned_step = PlannedStep {
            step: next_step,
            duration: config.base_step_duration + step_duration_modifier,
            start,
            target,
            swing_foot_height: config.base_foot_lift + foot_lift_modifier,
            swing_side: next_swing_foot,
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
    step_context: Res<StepContext>,
) {
    if *current == step_context.requested_gait {
        return;
    }

    info!(
        "switching requested gait to {:?}",
        step_context.requested_gait
    );
    commands.set_state(step_context.requested_gait);
}

fn plan_step(
    mut event: EventReader<FootSwitchedEvent>,
    mut step_context: ResMut<StepContext>,
    kinematics: Res<Kinematics>,
    config: Res<WalkingEngineConfig>,
) {
    let Some(event) = event.read().next() else {
        return;
    };

    let start = FootPositions::from_kinematics(event.new_swing, &kinematics, config.torso_offset);
    step_context.finish_step();
    step_context.plan_next_step(start, &config);
}

fn translation_weight(swing_travel: Vector2<f32>, turn_amount: f32, weights: Step) -> f32 {
    let translational = nalgebra::vector![
        weights.forward * swing_travel.x,
        weights.left * swing_travel.y,
    ]
    .norm();
    let rotational = weights.turn * turn_amount;
    translational + rotational
}

fn visualize_planned_step(dbg: DebugContext, cycle: Res<Cycle>, step_context: Res<StepContext>) {
    let planned = step_context.planned_step;
    dbg.log_with_cycle(
        "nao/planned_left_foot",
        *cycle,
        &rerun::Transform3D::update_fields()
            .with_translation(Vec3::from(planned.target.left.translation.vector))
            .with_quaternion(Quat::from(planned.target.left.rotation)),
    );

    dbg.log_with_cycle(
        "nao/planned_right_foot",
        *cycle,
        &rerun::Transform3D::update_fields()
            .with_translation(Vec3::from(planned.target.right.translation.vector))
            .with_quaternion(Quat::from(planned.target.right.rotation)),
    );
}
