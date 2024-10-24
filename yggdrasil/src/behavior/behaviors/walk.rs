use nalgebra::Point3;
use nidhogg::types::{FillExt, HeadJoints};

use crate::{
    behavior::engine::{Behavior, Context, Control},
    motion::walk::engine::Step,
    nao::Priority,
};

#[derive(Copy, Clone, PartialEq, Debug)]
pub struct Walk {
    pub step: Step,
    pub look_target: Option<Point3<f32>>,
}

impl Behavior for Walk {
    fn execute(&mut self, context: Context, control: &mut Control) {
        if let Some(point) = self.look_target {
            let look_at = context.pose.get_look_at_absolute(&point);
            control
                .nao_manager
                .set_head(look_at, HeadJoints::fill(0.5), Priority::High);
        }

        control.step_planner.clear_target();
        control.walking_engine.request_walk(self.step);
    }
}
