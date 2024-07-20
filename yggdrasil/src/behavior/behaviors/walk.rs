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
}

impl Walk {
    pub fn from_direction() -> Self {
        Self {
            step: Step {
                forward: 0.05,
                ..Default::default()
            },
        }
    }

    /// Walk to the left whilst turing to the right
    ///
    /// This kind of walk is meant for walking around an object.
    /// TODO: Make a formula that takes distance to calculate this step.
    pub fn circumnavigate_counterclockwise() -> Self {
        Self {
            step: Step {
                left: 0.03,
                turn: -0.33,
                ..Default::default()
            },
        }
    }

    pub fn circumnavigate_clockwise() -> Self {
        Self {
            step: Step {
                left: -0.03,
                turn: 0.33,
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
