use nalgebra::Point2;
use nidhogg::types::{FillExt, HeadJoints};

use crate::{
    behavior::engine::{Behavior, Context, Control},
    motion::walk::engine::Step,
    nao::manager::Priority,
};

#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct Walk {
    pub step: Step,
    pub target: Point2<f32>,
}

impl Behavior for Walk {
    fn execute(&mut self, context: Context, control: &mut Control) {
        control.step_planner.set_absolute_target(self.target);

        let look_at = context.pose.get_look_at_absolute(&self.target);

        control
            .nao_manager
            .set_head(look_at, HeadJoints::fill(0.5), Priority::High);
    }
}
