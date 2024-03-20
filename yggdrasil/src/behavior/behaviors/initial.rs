use crate::{
    behavior::engine::{Behavior, Context},
    config::general::layout::{InitialPositionsConfig, RobotPosition},
    kinematics,
    kinematics::FootOffset,
};
use miette::{miette, Result};
use nidhogg::{types::JointArray, NaoControlMessage};

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
    fn execute(&mut self, context: Context, control_message: &mut NaoControlMessage) -> Result<()> {
        let _position = get_position(
            &context.layout_config.initial_positions,
            context.robot_info.initial_player_number,
        )?;

        let mut builder = JointArray::<f32>::builder();

        // Stand straight
        let foot_offset = FootOffset {
            forward: 0.0,
            left: 0.0,
            turn: 0.0,
            hip_height: 0.0,
            lift: 0.0,
        };
        let (left_leg_joints, right_leg_joints) =
            kinematics::inverse::leg_angles(&foot_offset, &foot_offset);
        builder = builder
            .left_leg_joints(left_leg_joints)
            .right_leg_joints(right_leg_joints);

        // TODO:
        // Look at middle circle if feet touching the ground, otherwise look around
        // position
        control_message.position = builder.build();
        Ok(())
    }
}

fn get_position(
    initial_positions_config: &InitialPositionsConfig,
    player_number: i32,
) -> Result<RobotPosition> {
    match player_number {
        1 => Ok(initial_positions_config.one.clone()),
        2 => Ok(initial_positions_config.two.clone()),
        3 => Ok(initial_positions_config.three.clone()),
        4 => Ok(initial_positions_config.four.clone()),
        5 => Ok(initial_positions_config.five.clone()),
        _ => Err(miette!(
            "Invalid robot number! Cannot retrieve initial position."
        )),
    }
}
