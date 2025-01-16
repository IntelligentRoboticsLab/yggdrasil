use crate::{
    behavior::engine::{Behavior, Context, Control},
    nao::Priority,
};
use nidhogg::types::{color, FillExt, LeftLegJoints, LegJoints, RightEye, RightLegJoints};

// The robot shouldn't do anything while in unstiff state.
const UNSTIFF_PRIORITY: Priority = Priority::Critical;

/// This is often the starting behavior of the robot.
/// In this state the robot sits down, after which it unstiffens its legs, arms and head.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Sitting {
    /// Stores the initial leg position when ground contact is lost.
    locked_leg_position: Option<LegJoints<f32>>,
}

impl Sitting {
    fn capture_leg_position(context: &Context) -> LegJoints<f32> {
        let position = context.nao_state.position.clone();

        let left_leg_joints = LeftLegJoints::builder()
            .hip_yaw_pitch(position.left_hip_yaw_pitch)
            .hip_roll(position.left_hip_roll)
            .hip_pitch(position.left_hip_pitch)
            .knee_pitch(position.left_knee_pitch)
            .ankle_pitch(position.left_ankle_pitch)
            .ankle_roll(position.left_ankle_roll)
            .build();

        let right_leg_joints = RightLegJoints::builder()
            .hip_roll(position.right_hip_roll)
            .hip_pitch(position.right_hip_pitch)
            .knee_pitch(position.right_knee_pitch)
            .ankle_pitch(position.right_ankle_pitch)
            .ankle_roll(position.right_ankle_roll)
            .build();

        LegJoints::builder()
            .left_leg(left_leg_joints)
            .right_leg(right_leg_joints)
            .build()
    }
}

impl Behavior for Sitting {
    fn execute(&mut self, context: Context, control: &mut Control) {
        // Makes right eye blue.
        control
            .nao_manager
            .set_right_eye_led(RightEye::fill(color::f32::BLUE), Priority::default());

        if !control.walking_engine.is_sitting() {
            control.walking_engine.request_sit();
            control.nao_manager.unstiff_arms(UNSTIFF_PRIORITY);
            return;
        }

        if !context.contacts.ground && !control.keyframe_executor.is_motion_active() {
            // Read and store the current position when ground contact is lost.
            if self.locked_leg_position.is_none() {
                self.locked_leg_position = Some(Sitting::capture_leg_position(&context));
            }
            // Set the position
            if let Some(leg_positions) = self.locked_leg_position.as_ref() {
                control
                    .nao_manager
                    .stiff_sit(leg_positions.clone(), Priority::High);
            }

        // Resets locked position and makes robot floppy except for hip joints in sitting position.
        } else {
            self.locked_leg_position = None;
            control.nao_manager.unstiff_sit(UNSTIFF_PRIORITY);
        }

        control.nao_manager.unstiff_arms(UNSTIFF_PRIORITY);
    }
}
