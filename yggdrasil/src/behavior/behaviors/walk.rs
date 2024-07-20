use crate::{
    behavior::engine::{Behavior, Context, Control},
    motion::walk::engine::Step,
};

#[derive(Copy, Clone, PartialEq, Debug)]
pub struct Walk {
    pub step: Step,
}

impl Walk {
    pub fn forward() -> Self {
        Self {
            step: Step {
                forward: 0.05,
                ..Default::default()
            },
        }
    }
}

impl Behavior for Walk {
    fn execute(&mut self, _context: Context, control: &mut Control) {
        control.step_planner.clear_target();
        control.walking_engine.request_walk(self.step);
    }
}
