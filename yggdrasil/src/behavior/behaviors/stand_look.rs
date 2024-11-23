use std::time::Duration;

use crate::{
    behavior::engine::{Behavior, Context, Control},
    localization::RobotPose,
    nao::{HeadTarget, Priority},
};
use nalgebra::{Point2, Point3};
use nidhogg::types::{FillExt, HeadJoints};

const HEAD_STIFFNESS: f32 = 0.4;

/// Stand and look at a target point.
/// This is used for when the robot is in the Set state.
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct StandLookAt {
    pub target: Point2<f32>,
}

impl Behavior for StandLookAt {
    fn execute(&mut self, context: Context, control: &mut Control) {
        if let Some(ball) = context.ball_position {
            let ball_point3 = Point3::new(ball.x, ball.y, 0.0);
            let look_at = context.pose.get_look_at_absolute(&ball_point3);

            control.nao_manager.set_head_target(
                look_at,
                Duration::from_millis(1000),
            );
        } else {
            let target_point3 = Point3::new(-10.0, 0.0, 0.5);
            let look_at = context.pose.get_look_at_absolute(&target_point3);

            control.nao_manager.set_head_target(
                look_at,
                Duration::from_millis(1000),
            );
        }
        control.walking_engine.request_stand();
    }
}
