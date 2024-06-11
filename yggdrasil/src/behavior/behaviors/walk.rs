use nalgebra::Point2;

use crate::{
    behavior::engine::{Behavior, Context, Control},
    motion::walk::engine::Step,
};

#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct Walk {
    pub step: Step,
}

impl Behavior for Walk {
    fn execute(&mut self, _context: Context, control: &mut Control) {
        control
            .step_planner
            .set_absolute_target(Point2::new(0., 0.));
    }
}
