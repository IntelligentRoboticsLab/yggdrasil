use crate::{
    behavior::engine::{Behavior, Context},
    config::layout::RobotPosition,
    nao::manager::{NaoManager, Priority},
    walk::engine::WalkingEngine,
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
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct Initial;

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
    let stiffness = HeadJoints::fill(ROTATION_STIFFNESS);

    nao_manager.set_head(position, stiffness, Priority::default());
}

impl Behavior for Initial {
    fn execute(
        &mut self,
        context: Context,
        nao_manager: &mut NaoManager,
        _walking_engine: &mut WalkingEngine,
    ) {
        let player_num = context.player_config.player_number;
        let robot_position = &context.layout_config.initial_positions[player_num as usize];
        look_at_middle_circle(robot_position, nao_manager);

        walking_engine.request_stand();
    }
}
