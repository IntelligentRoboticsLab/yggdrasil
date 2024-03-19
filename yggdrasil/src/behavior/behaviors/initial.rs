use crate::{
    behavior::engine::{Behavior, Context},
    config::general::layout::RobotPosition,
    motion::arbiter::{MotionArbiter, Priority},
};
use nidhogg::types::HeadJoints;
use nidhogg::NaoControlMessage;

/// During a match the chest button is pressed before starting a match.
/// Once this is done, the robots are placed at the edge of the field from
/// which they will walk to their `Ready` positions.
///
/// This is the behaviour of the robot once the chest button is pressed.
/// In this state the robot will stand up straight and look at the middle
/// circle to make it easier to place the robot in the correct position.
/// After being placed at the side of the field the robot looks around to
/// improve localisation.
#[derive(Copy, Clone, Debug, Default)]
pub struct Initial;

impl Behavior for Initial {
    fn execute(
        &mut self,
        context: Context,
        motion_arbiter: &mut MotionArbiter,
        _control_message: &mut NaoControlMessage,
    ) {
        println!("In initial");

        // TODO
        // - Stand up straight (is done by default)
        // - Look at middle circle and interpolate this (wip)
        // - Once touching the ground for 5 sec
        //   turn head back and fourth 3x after pressing of chest button

        let player_num = context.yggdrasil_config.game_controller.player_number;
        let RobotPosition { x, y, .. } =
            context.layout_config.initial_positions[player_num as usize];

        let angle_rad = (x as f32 / y as f32).tan();
        let sign = if y > 0 { -1.0 } else { 1.0 };

        let position = HeadJoints {
            yaw: sign * angle_rad,
            pitch: 0.0,
        };

        let stiffness = HeadJoints {
            yaw: 0.5,
            pitch: 0.5,
        };

        motion_arbiter.set_head(position, stiffness, Priority::default());
    }
}
