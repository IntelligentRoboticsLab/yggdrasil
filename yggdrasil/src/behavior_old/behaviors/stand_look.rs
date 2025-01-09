use std::time::Duration;

use crate::{
    behavior_old::engine::{Behavior, Context, Control},
    localization::RobotPose,
    nao::{NaoManager, Priority},
};
use nalgebra::{Point2, Point3};

const HEAD_ROTATION_TIME: Duration = Duration::from_millis(500);

/// Stand and look at a target point.
/// This is used for when the robot is in the Set state.
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct StandLookAt {
    pub target: Point2<f32>,
}

impl Behavior for StandLookAt {
    fn execute(&mut self, context: Context, control: &mut Control) {
        let point3 = Point3::new(self.target.x, self.target.y, RobotPose::CAMERA_HEIGHT);
        let look_at = context.pose.get_look_at_absolute(&point3);

        control.nao_manager.set_head_target(
            look_at,
            HEAD_ROTATION_TIME,
            Priority::default(),
            NaoManager::HEAD_STIFFNESS,
        );
        control.walking_engine.request_stand();
    }
}
