use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DurationSeconds};
use std::time::{Duration, Instant};

use crate::{
    behavior::engine::{Behavior, Context},
    config::layout::RobotPosition,
    nao::manager::{NaoManager, Priority},
};
use nidhogg::types::{FillExt, HeadJoints};

const ROTATION_STIFFNESS: f32 = 0.3;

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
    // Measures time since placing the robot down
    placed_at: Option<Instant>,
}

/// Config struct containing parameters for the initial behavior.
#[serde_as]
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct InitialBehaviorConfig {
    // Controls how fast the robot moves its head back and forth while looking around
    pub head_rotation_speed: f32,
    // Controls how far to the left and right the robot looks while looking around, in radians.
    // If this value is one, the robot will look one radian to the left and one radian to the
    // right.
    pub head_pitch_max: f32,
    // Controls how far to the bottom the robot looks while looking around, in radians
    pub head_yaw_max: f32,
    // Duration after which the robot start observing after being picked up and placed
    #[serde_as(as = "DurationSeconds<u64>")]
    pub placed_duration_threshold: Duration,
}

fn at_starting_position(placed_at: Option<Instant>, threshold: Duration) -> bool {
    placed_at.is_some_and(|placed_at| placed_at.elapsed() > threshold)
}

fn look_around(
    nao_manager: &mut NaoManager,
    placed_time: Instant,
    placed_time_offsett: Duration,
    rotation_speed: f32,
    yaw_multiplier: f32,
    pitch_multiplier: f32,
) {
    let at_starting_pos_for = (placed_time.elapsed() - placed_time_offsett).as_secs_f32();
    // Used to parameterize the yaw and pitch angles, multiplying with a large
    // rotation speed will make the rotation go faster.
    let x = at_starting_pos_for * rotation_speed;
    let yaw = (x).sin() * yaw_multiplier;
    let pitch = (x * 2.0 + std::f32::consts::FRAC_PI_2).sin().max(0.0) * pitch_multiplier;

    let position = HeadJoints { yaw, pitch };
    let stiffness = HeadJoints::fill(ROTATION_STIFFNESS);

    nao_manager.set_head(position, stiffness, Priority::default());
}

fn look_at_middle_circle(robot_position: &RobotPosition, nao_manager: &mut NaoManager) {
    // Transform center point from world space to robot space.
    let sign = robot_position.y.signum() as f32;
    let transformed_center_x = robot_position.x as f32 * sign;
    let transformed_center_y = robot_position.y as f32 * sign;

    // Compute angle and then convert to the nek yaw, this angle is dependent on
    // which side of the field the robot is located.
    let angle = (transformed_center_y / transformed_center_x).atan();
    let yaw = (std::f32::consts::FRAC_PI_2 + angle * sign) * sign;

    let position = HeadJoints { yaw, pitch: 0.0 };
    let stiffness = HeadJoints::fill(0.3);

    nao_manager.set_head(position, stiffness, Priority::default());
}

impl Behavior for Initial {
    fn execute(&mut self, context: Context, nao_manager: &mut NaoManager) {
        let InitialBehaviorConfig {
            head_rotation_speed,
            head_pitch_max: head_pitch_multiplier,
            head_yaw_max: head_yaw_multiplier,
            placed_duration_threshold,
        } = context.behavior_config.initial_behaviour;

        if context.contacts.ground {
            self.placed_at.get_or_insert(Instant::now());
        } else {
            self.placed_at = None;
        }

        if !at_starting_position(self.placed_at, placed_duration_threshold) {
            let player_num = context.yggdrasil_config.game_controller.player_number;
            let robot_position = &context.layout_config.initial_positions[player_num as usize];
            look_at_middle_circle(robot_position, nao_manager);
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
