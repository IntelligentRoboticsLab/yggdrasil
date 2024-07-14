use nalgebra::Point2;
use nidhogg::types::{color, FillExt, HeadJoints};

use crate::{
    behavior::engine::{Behavior, Context, Control},
    core::debug::DebugContext,
    motion::step_planner::Target,
    nao::manager::Priority,
};

#[derive(Copy, Clone, PartialEq, Debug)]
pub struct Walk {
    pub target: Target,
}

fn log_target(target: &Target, debug_context: &mut DebugContext) {
    if let Some(rotation) = target.rotation {
        let direction = rotation.transform_point(&Point2::new(0.1, 0.0));

        debug_context
            .log_arrows3d_with_color(
                "odometry/target",
                &[(direction.x, direction.y, 0.0)],
                &[(target.position.x, target.position.y, 0.0)],
                color::u8::RED,
            )
            .unwrap();
    } else {
        debug_context
            .log_points_3d_with_color_and_radius(
                "odometry/target",
                &[(target.position.x, target.position.y, 0.0)],
                color::u8::RED,
                0.04,
            )
            .unwrap();
    }
}

impl Behavior for Walk {
    fn execute(&mut self, context: Context, control: &mut Control) {
        let target_point = self.target.position;

        let look_at = context.pose.get_look_at_absolute(&target_point);
        control
            .nao_manager
            .set_head(look_at, HeadJoints::fill(0.5), Priority::High);
        log_target(&self.target, control.debug_context);

        if control
            .step_planner
            .current_absolute_target()
            .is_some_and(|target| target != &self.target)
        {
            control.step_planner.clear_target();
        }

        control
            .step_planner
            .set_absolute_target_if_unset(self.target);
        if let Some(step) = control.step_planner.plan(context.pose) {
            control.walking_engine.request_walk(step);
        } else {
            control.walking_engine.request_stand();
        }
    }
}
