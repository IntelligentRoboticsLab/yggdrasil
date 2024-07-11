use nalgebra::Point2;
use nidhogg::types::{color, FillExt, HeadJoints};

use crate::{
    behavior::engine::{Behavior, Context, Control},
    nao::manager::Priority,
};

#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct Walk {
    pub target: Point2<f32>,
}

impl Behavior for Walk {
    fn execute(&mut self, context: Context, control: &mut Control) {
        let look_at = context.pose.get_look_at_absolute(&self.target);
        control
            .nao_manager
            .set_head(look_at, HeadJoints::fill(0.5), Priority::default());
        control
            .debug_context
            .log_points_3d_with_color_and_radius(
                "/odometry/target",
                &[(self.target.x, self.target.y, 0.)],
                color::u8::RED,
                0.05,
            )
            .unwrap();

        control.step_planner.set_absolute_target(self.target);
        if let Some(step) = control.step_planner.plan(context.pose) {
            control.walking_engine.request_walk(step);
        } else {
            control.walking_engine.request_stand();
            control.step_planner.clear_target();
        }
    }
}
