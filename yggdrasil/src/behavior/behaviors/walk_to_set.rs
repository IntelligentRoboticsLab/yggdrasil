use nidhogg::types::{FillExt, HeadJoints};

use nalgebra::{Point2, Point3};

use crate::{
    behavior::engine::{Behavior, Context, Control},
    localization::RobotPose,
    motion::step_planner::Target,
    nao::Priority,
};

/// To prevent the keeper from walking into the goalpost, we use this position for a better approach.
const KEEPER_PRE_SET_POS: Target = Target {
    position: Point2::new(-2.85, 0.0),
    rotation: None,
};

/// Walk to the set position of the robot.
/// Only the keeper will first walk to the pre-set position before walking to the set position.
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct WalkToSet {
    pub is_keeper: bool,
}

impl Behavior for WalkToSet {
    fn execute(&mut self, context: Context, control: &mut Control) {
        let set_robot_position = context
            .layout_config
            .set_positions
            .player(context.player_config.player_number);

        let set_position: Point2<f32> = set_robot_position.isometry.translation.vector.into();

        let look_at =
            // Setting z to default `CAMERA_HEIGHT` (looking straight ahead).
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

        let reached_pre_set = !control.step_planner.has_target()
            || (control
                .step_planner
                .current_absolute_target()
                .is_some_and(|target| target == &KEEPER_PRE_SET_POS)
                && !control.step_planner.reached_target());

        if self.is_keeper && reached_pre_set {
            control.step_planner.set_absolute_target(KEEPER_PRE_SET_POS);
        } else {
            control.step_planner.set_absolute_target(target);
        }

        if let Some(step) = control.step_planner.plan(context.pose) {
            control.walking_engine.request_walk(step);
        } else {
            let look_at = context.pose.get_look_at_absolute(&Point3::origin());
            control
                .nao_manager
                .set_head(look_at, HeadJoints::fill(0.5), Priority::default());

            control.walking_engine.request_stand();
        }
    }
}
