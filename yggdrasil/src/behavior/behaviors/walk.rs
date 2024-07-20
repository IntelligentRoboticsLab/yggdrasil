use nalgebra::Point2;

use crate::{
    behavior::engine::{Behavior, Context, Control},
    motion::walk::engine::Step,
};

pub enum Direction {
    Forward,
    Left,
    Right,
    Back,
    CircumnavClockWise,
    CircumnavCounterClockWise,
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub struct Walk {
    pub step: Step,
    pub look_target: Option<Point2<f32>>,
}

impl Walk {
    pub fn from_direction() -> Self {
        Self {
            step: Step {
                forward: 0.05,
                ..Default::default()
            },
            look_target: None,
        }
    }
}

impl Behavior for Walk {
    fn execute(&mut self, _context: Context, control: &mut Control) {
        control.step_planner.clear_target();
        control.walking_engine.request_walk(self.step);
    }
}
