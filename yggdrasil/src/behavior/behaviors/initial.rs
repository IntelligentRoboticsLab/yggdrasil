use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DurationSeconds};
use std::time::{Duration, Instant};

use crate::{
    behavior::engine::{Behavior, Context},
    config::layout::RobotPosition,
    nao::manager::{NaoManager, Priority},
};
use nidhogg::types::{FillExt, HeadJoints};

/// During a match the chest button is pressed before starting a match.
/// Once this is done, the robots are placed at the edge of the field from
/// which they will walk to their `Ready` positions.
///
/// This is the behaviour of the robot once the chest button is pressed.
/// In this state the robot will stand up straight and look at the middle
/// circle to make it easier to place the robot in the correct position.
/// After being placed at the side of the field the robot looks around to
/// improve localisation and start detecting lines, etc.
#[derive(Copy, Clone, Debug, Default)]
pub struct Initial {
    // Keeps track of whether the robot is currently lifted or not
    lifted: bool,
    // Measures time since placing the robot down
    placed_at: Option<Instant>,
    // Keeps track of whether the robot is ready to start looking around
    at_starting_position: bool,
}

/// Config struct containing parameters for the initial behavior.
#[serde_as]
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct InitialBehaviorConfig {
    // Controls how fast the robot moves its head back and forth while looking around
    pub head_rotation_speed: f32,
    // Controls how far to the left and right the robot looks while looking around
    pub head_pitch_multiplier: f32,
    // Controls how far to the bottom the robot looks while looking around
    pub head_yaw_multiplier: f32,
    // Duration after which the robot start observing after being picked up and placed
    #[serde_as(as = "DurationSeconds<u64>")]
    pub placed_duration_threshold: Duration,
}

fn look_around(
    nao_manager: &mut NaoManager,
    placed_time: Instant,
    placed_time_offsett: Duration,
    rotation_speed: f32,
    yaw_multiplier: f32,
    pitch_multiplier: f32,
) {
    let time = (placed_time.elapsed() - placed_time_offsett).as_millis() as f32 / 1000_f32;
    let x = time * rotation_speed;
    let yaw = (x).sin() * yaw_multiplier;
    let pitch = (x * 2.0 + std::f32::consts::FRAC_PI_2).sin().max(0.0) * pitch_multiplier;

    let position = HeadJoints { yaw, pitch };
    let stiffness = HeadJoints::fill(0.3);

    nao_manager.set_head(position, stiffness, Priority::default());
}

impl Behavior for Initial {
    fn execute(&mut self, context: Context, nao_manager: &mut NaoManager) {
        let InitialBehaviorConfig {
            head_rotation_speed,
            head_pitch_multiplier,
            head_yaw_multiplier,
            placed_duration_threshold,
        } = context.behavior_config.initial_behaviour;

        if context.contacts.ground {
            if self.lifted {
                self.placed_at = Some(Instant::now());
                self.lifted = false;
            } else if self.placed_at.is_some()
                && self.placed_at.unwrap().elapsed() > placed_duration_threshold
            {
                self.at_starting_position = true;
            }
        } else {
            self.placed_at = None;
            self.at_starting_position = false;
            self.lifted = true;
        }

        if !self.at_starting_position {
            let player_num = context.yggdrasil_config.game_controller.player_number;
            let RobotPosition { x, y, .. } =
                context.layout_config.initial_positions[player_num as usize];

            // Transform center point from world space to robot space.
            let sign = y.signum() as f32;
            let transformed_center_x = x as f32 * sign;
            let transformed_center_y = y as f32 * sign;

            // Compute angle and then convert to the nek yaw.
            let angle = (transformed_center_y / transformed_center_x).atan();
            let yaw = (std::f32::consts::FRAC_PI_2 + angle * sign) * sign;

            let position = HeadJoints { yaw, pitch: 0.0 };
            let stiffness = HeadJoints::fill(0.3);

            nao_manager.set_head(position, stiffness, Priority::default());
        } else {
            look_around(
                nao_manager,
                self.placed_at.unwrap(),
                placed_duration_threshold,
                head_rotation_speed,
                head_yaw_multiplier,
                head_pitch_multiplier,
            );
        }
    }
}
