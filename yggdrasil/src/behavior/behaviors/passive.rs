use crate::{
    behavior::engine::{Behavior, Context},
    nao::manager::{NaoManager, Priority},
};
use nidhogg::types::{color, ArmJoints, FillExt, HeadJoints, JointArray, LegJoints, RightEye};

const DEFAULT_PASSIVE_STIFFNESS: f32 = 0.8;
const DEFAULT_PASSIVE_PRIORITY: Priority = Priority::Medium;

/// This is the default behavior of the robot.
/// In this state the robot does nothing and all motors are turned off.
/// In this state the robot has a blue right eye.
#[derive(Copy, Clone, Debug, Default)]
pub struct Passive {
    pub floppy: bool,
}

impl Behavior for Passive {
    fn execute(&mut self, context: Context, nao_manager: &mut NaoManager) {
        if context.head_buttons.middle.is_pressed() {
            self.floppy = true;
        }

        if self.floppy {
            // TODO: sit down
            nao_manager
                .unstiff_legs(DEFAULT_PASSIVE_PRIORITY)
                .unstiff_arms(DEFAULT_PASSIVE_PRIORITY)
                .unstiff_head(DEFAULT_PASSIVE_PRIORITY);
        } else {
            set_initial_joint_values(
                context.robot_info.initial_joint_positions.clone(),
                nao_manager,
            )
        }

        // Makes right eye blue.
        nao_manager.set_right_eye_led(RightEye::fill(color::f32::BLUE), Priority::default());
    }
}

fn set_initial_joint_values(
    initial_joint_positions: JointArray<f32>,
    nao_manager: &mut NaoManager,
) {
    let head_joints = initial_joint_positions.head_joints();
    nao_manager.set_head(
        head_joints,
        HeadJoints::fill(DEFAULT_PASSIVE_STIFFNESS),
        DEFAULT_PASSIVE_PRIORITY,
    );

    let left_arm_joints = initial_joint_positions.left_arm_joints();
    let right_arm_joints = initial_joint_positions.right_arm_joints();
    nao_manager.set_arms(
        ArmJoints::builder()
            .left_arm(left_arm_joints)
            .right_arm(right_arm_joints)
            .build(),
        ArmJoints::fill(DEFAULT_PASSIVE_STIFFNESS),
        DEFAULT_PASSIVE_PRIORITY,
    );

    let left_leg_joints = initial_joint_positions.left_leg_joints();
    let right_leg_joints = initial_joint_positions.right_leg_joints();
    nao_manager.set_legs(
        LegJoints::builder()
            .left_leg(left_leg_joints)
            .right_leg(right_leg_joints)
            .build(),
        LegJoints::fill(DEFAULT_PASSIVE_STIFFNESS),
        DEFAULT_PASSIVE_PRIORITY,
    );
}
