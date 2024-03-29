use crate::{
    behavior::engine::{Behavior, Context},
    config::general::layout::RobotPosition,
    nao::manager::{NaoManager, Priority},
};
use nidhogg::types::{FillExt, HeadJoints};
use std::time::{Duration, Instant};

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
    lifted: bool,
    placed_at: Option<Instant>,
    at_starting_position: bool,
}

// TODO: tweak these values
const ROTATION_SPEED: f32 = 1_f32;
const PLACED_DURATION_THRESHOLD: u64 = 30;

fn look_around(nao_manager: &mut NaoManager, placed_time: Instant) {
    let yaw = (placed_time.elapsed().as_millis() as f32 * 1000_f32 * ROTATION_SPEED).sin();
    let position = HeadJoints { yaw, pitch: 0.0 };
    let stiffness = HeadJoints::fill(0.3);

    nao_manager.set_head(position, stiffness, Priority::default());
}

impl Behavior for Initial {
    fn execute(&mut self, context: Context, nao_manager: &mut NaoManager) {
        match context.contacts.ground {
            true => {
                if self.lifted {
                    self.placed_at = Some(Instant::now());
                    self.lifted = false;
                } else if self.placed_at.is_some()
                    && self.placed_at.unwrap().elapsed()
                        > Duration::from_secs(PLACED_DURATION_THRESHOLD)
                {
                    self.at_starting_position = true;
                }
            }
            false => {
                self.placed_at = None;
                self.at_starting_position = false;
                self.lifted = true;
            }
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
            let yaw = -(std::f32::consts::FRAC_PI_2 + angle * sign) * sign;

            let position = HeadJoints { yaw, pitch: 0.0 };
            let stiffness = HeadJoints::fill(0.3);

            nao_manager.set_head(position, stiffness, Priority::default());
        } else {
            look_around(nao_manager, self.placed_at.unwrap());
        }
    }
}
