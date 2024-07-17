use crate::{
    behavior::engine::{Behavior, Context, Control},
    motion::keyframe::MotionType,
    nao::manager::Priority,
    sensor::falling::{FallState, LyingDirection},
};

/// Behavior dedicated to handling the getup sequence of the robot.
/// The behavior will be entered once the robot is confirmed to be lying down,
/// this will execute the appropriate standup motion after which the robot will return
/// to the appropriate next behavior.
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct Standup {
    completed: bool,
}

impl Standup {
    pub fn completed(&self) -> bool {
        self.completed
    }
}

impl Behavior for Standup {
    fn execute(&mut self, context: Context, control: &mut Control) {
        // check the direction the robot is lying and execute the appropriate motion
        match context.fall_state {
            FallState::Lying(LyingDirection::FacingDown) => {
                control
                    .keyframe_executor
                    .start_new_motion(MotionType::StandupStomach, Priority::High);
            }
            FallState::Lying(LyingDirection::FacingUp) => {
                control
                    .keyframe_executor
                    .start_new_motion(MotionType::StandupBack, Priority::High);
            }
            // if we are not lying down anymore, either standing up or falling, we do not execute any motion
            _ => {}
        }

        self.completed = !control.keyframe_executor.is_motion_active();
    }
}
