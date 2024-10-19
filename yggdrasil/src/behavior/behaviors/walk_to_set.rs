use nidhogg::types::{FillExt, HeadJoints};

use nalgebra::{Point2, Point3};

use crate::{
    behavior::engine::{Behavior, Context, Control}, localization::RobotPose, motion::step_planner::Target, nao::Priority
};

#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct WalkToSet;

impl Behavior for WalkToSet {
    fn execute(&mut self, context: Context, control: &mut Control) {
        let set_robot_position = context
            .layout_config
            .set_positions
            .player(context.player_config.player_number);

        let set_position: Point2<f32> = set_robot_position.isometry.translation.vector.into();

        let look_at =
            // Setting z to default 0.5 (looking straight ahead)
            context
                .pose
                .get_look_at_absolute(&Point3::new(set_position.x, set_position.y, RobotPose::CAMERA_HEIGHT));
        control
            .nao_manager
            .set_head(look_at, HeadJoints::fill(0.5), Priority::default());

        let target = Target {
            position: set_robot_position.isometry.translation.vector.into(),
            rotation: Some(set_robot_position.isometry.rotation),
        };

        control.step_planner.set_absolute_target_if_unset(target);
        if let Some(step) = control.step_planner.plan(context.pose) {
            control.walking_engine.request_walk(step);
        } else {
            control.walking_engine.request_stand();
        }
    }
}
