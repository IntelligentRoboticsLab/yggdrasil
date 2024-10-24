use nalgebra::{Point2, Point3};
use nidhogg::types::{FillExt, HeadJoints};

use crate::{
    behavior::engine::{Behavior, Context, Control},
    motion::walk::engine::Step,
    nao::Priority,
};

#[derive(Copy, Clone, PartialEq, Debug)]
pub struct Walk {
    pub step: Step,
    pub look_target: Option<Point2<f32>>,
}

impl Behavior for Walk {
    fn execute(&mut self, context: Context, control: &mut Control) {
        if let Some(point) = self.look_target {
            let target_point = Point3::new(point.x, point.y, 0.0);

            let look_at = context.pose.get_look_at_absolute(&target_point);
            control
                .nao_manager
                .set_head(look_at, HeadJoints::fill(0.5), Priority::High);
        }

        control.step_planner.clear_target();
        control.walking_engine.request_walk(self.step);
    }
}
