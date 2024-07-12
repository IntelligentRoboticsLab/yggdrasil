use nalgebra::{Isometry2, Point2};
use nidhogg::types::{color, FillExt, HeadJoints};

use crate::{
    behavior::engine::{Behavior, Context, Control},
    core::debug::DebugContext,
    nao::manager::Priority,
};

#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct Walk {
    pub target: Isometry2<f32>,
}

fn log_target(target: &Isometry2<f32>, debug_context: &mut DebugContext) {
    let origin = target.translation.vector;
    let direction = target.rotation.transform_point(&Point2::new(0.1, 0.0));

    debug_context
        .log_arrows3d_with_color(
            "odometry/target",
            &[(direction.x, direction.y, 0.0)],
            &[(origin.x, origin.y, 0.0)],
            color::u8::RED,
        )
        .unwrap();
}

impl Behavior for Walk {
    fn execute(&mut self, context: Context, control: &mut Control) {
        let target_point = self
            .target
            .translation
            .transform_point(&Point2::new(0.0, 0.0));

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
