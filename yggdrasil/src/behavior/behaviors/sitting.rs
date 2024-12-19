use crate::{
    behavior::engine::{Behavior, Context, Control},
    nao::Priority,
};
use nidhogg::types::{color, FillExt, LeftLegJoints, RightLegJoints, LegJoints, RightEye};
// use std::{time::{Duration, Instant}};

// The robot shouldn't do anything while in unstiff state.
const UNSTIFF_PRIORITY: Priority = Priority::Critical;

/// This is often the starting behavior of the robot.
/// In this state the robot sits down, after which it unstiffens its legs, arms and head.
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct Sitting;


impl Behavior for Sitting {
    fn execute(&mut self, context: Context, control: &mut Control) {
        // Makes right eye blue.
        control
            .nao_manager
            .set_right_eye_led(RightEye::fill(color::f32::BLUE), Priority::default());

        if control.walking_engine.is_sitting() {
            // Makes robot floppy except for hip joints, locked in sitting position.
            // control.nao_manager.unstiff_sit(UNSTIFF_PRIORITY);
            // -----------------------------------------------
            // Lock sit upon losing ground contact
            println!("Ground contact: {}", context.contacts.ground);
            if !context.contacts.ground && !control.keyframe_executor.is_motion_active(){

                println!("Stay sit");
                
                // read position
                let robot_position = context.nao_state.position.clone();

                println!("Robot pos: {:?}", robot_position);
            
                // set position
                let left_leg_joints = LeftLegJoints::builder()
                    .hip_yaw_pitch(robot_position.left_hip_yaw_pitch)
                    .hip_roll(robot_position.left_hip_roll)
                    .hip_pitch(robot_position.left_hip_pitch)
                    .knee_pitch(robot_position.left_knee_pitch)
                    .ankle_pitch(robot_position.left_ankle_pitch)
                    .ankle_roll(robot_position.left_ankle_roll)
                    .build();

                let right_leg_joints = RightLegJoints::builder()
                    .hip_roll(robot_position.right_hip_roll)
                    .hip_pitch(robot_position.right_hip_pitch)
                    .knee_pitch(robot_position.right_knee_pitch)
                    .ankle_pitch(robot_position.right_ankle_pitch)
                    .ankle_roll(robot_position.right_ankle_roll)
                    .build();

                let leg_positions = LegJoints::builder()
                    .left_leg(left_leg_joints)
                    .right_leg(right_leg_joints)
                    .build();

                println!("Leg pos: {:?}", leg_positions);

                // control.nao_manager.set_legs(leg_positions, leg_stiffness, Priority::default());
                // control.nao_manager.set_legs(leg_positions, leg_stiffness, UNSTIFF_PRIORITY);
                // control.nao_manager.stiff_sit(leg_positions, Priority::default());
                control.nao_manager.stiff_sit(leg_positions, Priority::Critical);
                }

            // Makes robot floppy except for hip joints, locked in sitting position.
            else {
                println!("Floppy");
                control.nao_manager.unstiff_sit(UNSTIFF_PRIORITY);
            }
        } else {
            control.walking_engine.request_sit();
        }

        control.nao_manager.unstiff_arms(UNSTIFF_PRIORITY);
    }
}
