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
    FootSwitchedEvent, Side,
};
use bevy::prelude::*;
use nalgebra::Vector2;
use rerun::external::glam::{Quat, Vec3};

// Define kick variant type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KickVariant {
    Forward,
    Turn,
    Side,
}

// Define the kick sequence information
#[derive(Debug, Clone)]
pub struct KickSequence {
    pub variant: KickVariant,
    pub kicking_side: Side,
    pub strength: f32,
    pub step_index: usize,
    pub total_steps: usize,
}

pub(super) struct StepContextPlugin;

impl Plugin for StepContextPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostStartup, setup_step_visualizer);
        app.add_systems(
            PreUpdate,
            sync_gait_request.in_set(WalkingEngineSet::Prepare),
        );

        app.add_systems(
            PreUpdate,
            plan_step
                .run_if(on_event::<FootSwitchedEvent>)
                .in_set(WalkingEngineSet::PlanStep)
                .after(crate::kinematics::update_kinematics),
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
    pub active_kick: Option<KickSequence>,
}

impl StepContext {
    #[must_use]
    pub fn init(gait: Gait, last_step: PlannedStep) -> Self {
        Self {
            requested_gait: gait,
            requested_step: Step::default(),
            last_step,
            planned_step: last_step,
            active_kick: None,
        }
    }

    pub fn request_sit(&mut self) {
        self.requested_gait = Gait::Sitting;
        self.last_step = PlannedStep {
            swing_side: self.last_step.swing_side,
            ..Default::default()
        };
        self.requested_step = Step::default();
        self.active_kick = None;
    }

    pub fn request_stand(&mut self) {
        match self.requested_gait {
            Gait::Walking => {
                self.requested_gait = Gait::Stopping;
            }
            Gait::Sitting | Gait::Standing | Gait::Starting | Gait::Stopping | Gait::Kicking => {
                // the robot can immediately move to Gait::Stand
                self.requested_gait = Gait::Standing;
            }
        }

        self.last_step = PlannedStep {
            swing_side: self.last_step.swing_side,
            ..Default::default()
        };
        self.requested_step = Step::default();
        self.active_kick = None;
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
            Gait::Starting | Gait::Walking | Gait::Stopping | Gait::Kicking => {
                // the robot is currently starting, stopping or walking already
                // so we just change the requested step
                self.requested_step = step;

                // If we're kicking and received a walk request, finish the kick sequence after the current step
                if self.requested_gait == Gait::Kicking {
                    if let Some(kick) = &mut self.active_kick {
                        kick.total_steps = kick.step_index + 1; // End after current step
                    }
                }
            }
        }
    }

    // New method to request a kick
    pub fn request_kick(&mut self, variant: KickVariant, kicking_side: Side, strength: f32) {
        // Can only kick from standing, starting or walking
        match self.requested_gait {
            Gait::Sitting => error!(
                "Cannot request kick while sitting! Call StepManager::request_stand() first!"
            ),
            Gait::Standing => {
                // Need to start walking first
                self.requested_gait = Gait::Starting;
                self.requested_step = Step::default(); // Use default step for starting

                // Set up kick to execute after starting
                self.active_kick = Some(KickSequence {
                    variant,
                    kicking_side,
                    strength,
                    step_index: 0,
                    total_steps: get_kick_steps_count(variant),
                });
            }
            Gait::Starting | Gait::Walking | Gait::Stopping => {
                // Set up the kick to execute on next foot switch
                self.active_kick = Some(KickSequence {
                    variant,
                    kicking_side,
                    strength,
                    step_index: 0,
                    total_steps: get_kick_steps_count(variant),
                });
            }
            Gait::Kicking => {
                // Already kicking, queue a new kick
                self.active_kick = Some(KickSequence {
                    variant,
                    kicking_side,
                    strength,
                    step_index: 0,
                    total_steps: get_kick_steps_count(variant),
                });
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

    pub(super) fn finish_stopping_step(&mut self, step: PlannedStep) {
        self.last_step = step;
        self.requested_gait = Gait::Standing;
    }

    // Check if we need to transition out of kicking
    pub(super) fn check_kick_progress(&mut self) -> bool {
        if let Some(kick) = &mut self.active_kick {
            kick.step_index += 1;

            if kick.step_index >= kick.total_steps {
                // Kick sequence complete
                self.requested_gait = Gait::Walking;
                self.active_kick = None;
                true
            } else {
                // Continue kicking
                false
            }
        } else {
            // No active kick
            false
        }
    }

    pub fn plan_next_step(&mut self, start: FootPositions, config: &WalkingEngineConfig) {
        let next_swing_foot = self.last_step.swing_side.opposite();

        // If we have an active kick and are about to use the kicking foot as support,
        // we need a preparation step
        if let Some(kick) = &self.active_kick {
            if self.requested_gait != Gait::Kicking && next_swing_foot == kick.kicking_side {
                // Need a preparation step - this ensures the kicking foot becomes the swing foot
                let prep_step = create_preparation_step(kick.kicking_side);

                let target = FootPositions::from_target(next_swing_foot, &prep_step);

                self.planned_step = PlannedStep {
                    step: prep_step,
                    duration: config.base_step_duration,
                    start,
                    target,
                    swing_foot_height: config.base_foot_lift,
                    swing_side: next_swing_foot,
                };

                return;
            } else if self.requested_gait == Gait::Kicking {
                // We're in kicking mode, generate the appropriate kick step
                let kick_step = generate_kick_step(
                    kick.variant,
                    kick.kicking_side,
                    kick.strength,
                    kick.step_index,
                );

                let target = FootPositions::from_target(next_swing_foot, &kick_step);

                // Apply kick-specific foot lift height
                let swing_foot_height = match kick.variant {
                    KickVariant::Forward => match kick.step_index {
                        0 => 0.015,
                        1 => 0.02 * kick.strength,
                        2 => 0.015,
                        _ => 0.01,
                    },
                    KickVariant::Turn => match kick.step_index {
                        0 => 0.01,
                        1 => 0.02 * kick.strength,
                        _ => 0.01,
                    },
                    KickVariant::Side => match kick.step_index {
                        0 => 0.01,
                        1 => 0.02 * kick.strength,
                        _ => 0.01,
                    },
                };

                // For main kick step, use longer duration
                let duration = if kick.step_index == 1 {
                    Duration::from_millis(280) // Slightly longer for the main kick motion
                } else {
                    Duration::from_millis(260)
                };

                self.planned_step = PlannedStep {
                    step: kick_step,
                    duration,
                    start,
                    target,
                    swing_foot_height,
                    swing_side: next_swing_foot,
                };

                return;
            }
        }

        // Normal walking step planning
        let delta_step = (self.requested_step - self.last_step.step)
            .clamp(-config.max_acceleration, config.max_acceleration);

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

    // Utility to check if the support side is correct for kicking
    pub fn is_ready_for_kick(&self) -> bool {
        if let Some(kick) = &self.active_kick {
            // The swing foot needs to be the kicking foot
            self.last_step.swing_side.opposite() == kick.kicking_side
        } else {
            false
        }
    }
}

// Helper function to create a preparation step before a kick
fn create_preparation_step(kicking_side: Side) -> Step {
    // Use a small neutral step to ensure we're on the correct support foot
    match kicking_side {
        Side::Left => Step {
            forward: 0.01,
            left: -0.05, // Step slightly right to prepare for left kick
            turn: 0.0,
        },
        Side::Right => Step {
            forward: 0.01,
            left: 0.05, // Step slightly left to prepare for right kick
            turn: 0.0,
        },
    }
}

// Helper function to determine how many steps a kick sequence requires
fn get_kick_steps_count(variant: KickVariant) -> usize {
    match variant {
        KickVariant::Forward => 4,
        KickVariant::Turn => 3,
        KickVariant::Side => 2,
    }
}

// Generate appropriate step parameters for each phase of the kick
fn generate_kick_step(
    variant: KickVariant,
    kicking_side: Side,
    strength: f32,
    step_index: usize,
) -> Step {
    match variant {
        KickVariant::Forward => {
            match step_index {
                0 => {
                    // First step: small step forward to position
                    Step {
                        forward: 0.04 * strength,
                        left: 0.0,
                        turn: 0.0,
                    }
                }
                1 => {
                    // Second step: main kick motion
                    Step {
                        forward: 0.08 * strength,
                        left: 0.0,
                        turn: 0.0,
                    }
                }
                _ => {
                    // Recovery steps
                    Step::default()
                }
            }
        }
        KickVariant::Turn => {
            match step_index {
                0 => {
                    // First step: turn to position
                    let turn_value = -0.8 * strength; // Negative turn for positioning
                    Step {
                        forward: 0.0,
                        left: 0.0,
                        turn: if kicking_side == Side::Right {
                            -turn_value
                        } else {
                            turn_value
                        },
                    }
                }
                1 => {
                    // Second step: main kick motion with turn
                    let turn_value = -0.2 * strength;
                    Step {
                        forward: 0.06 * strength,
                        left: 0.0,
                        turn: if kicking_side == Side::Right {
                            -turn_value
                        } else {
                            turn_value
                        },
                    }
                }
                _ => {
                    // Recovery step
                    Step::default()
                }
            }
        }
        KickVariant::Side => {
            match step_index {
                0 => {
                    // First step: preparation
                    Step::default()
                }
                1 => {
                    // Second step: side kick
                    let side_value = 0.12 * strength;
                    Step {
                        forward: 0.0,
                        left: if kicking_side == Side::Right {
                            -side_value
                        } else {
                            side_value
                        },
                        turn: 0.0,
                    }
                }
                _ => Step::default(),
            }
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
