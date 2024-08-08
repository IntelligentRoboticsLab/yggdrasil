use std::{ops::Neg, time::Duration};

use crate::{kinematics::RobotKinematics, motion::walk::engine::Side};

use super::feet::Feet;

/// A step that can be taken by the walking engine.
#[derive(Default, Debug, Clone, Copy, PartialEq)]
pub struct StepRequest {
    /// The forward component of the step, in meters.
    pub forward: f32,
    /// The left component of the step, in meters.
    pub left: f32,
    /// The turn component of the step, in radians.
    pub turn: f32,
}

impl StepRequest {
    /// Create a new step with the given forward, left, and turn components.
    pub fn new(forward: f32, left: f32, turn: f32) -> Self {
        Self {
            forward,
            left,
            turn,
        }
    }

    /// Restrict the step to a certain range.
    pub fn clamp(mut self, min: Self, max: StepRequest) -> Self {
        self.forward = self.forward.clamp(min.forward, max.forward);
        self.left = self.left.clamp(min.left, max.left);
        self.turn = self.turn.clamp(min.turn, max.turn);

        self
    }

    /// Clamp the step to the anatomic limits of the robot.
    pub fn clamp_anatomic(self, swing_side: Side, max_inside_turn: f32) -> Self {
        let sideways_dir = if self.left.is_sign_positive() {
            Side::Left
        } else {
            Side::Right
        };

        let clamped_sideways = if sideways_dir == swing_side {
            self.left
        } else {
            0.0
        };

        let turn_direction = if self.turn.is_sign_positive() {
            Side::Left
        } else {
            Side::Right
        };

        let clamped_turn = if turn_direction != swing_side {
            self.turn
        } else {
            self.turn.clamp(-max_inside_turn, max_inside_turn)
        };

        Self {
            forward: self.forward,
            left: clamped_sideways,
            turn: clamped_turn,
        }
    }
}

impl Neg for StepRequest {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self {
            forward: -self.forward,
            left: -self.left,
            turn: -self.turn,
        }
    }
}

#[derive(Default, Debug, Clone, Copy, PartialEq)]
pub struct PlannedStep {
    pub request: StepRequest,
    pub duration: Duration,
    pub start: Feet,
    pub end: Feet,
    pub foot_height: f32,
}

impl PlannedStep {
    pub fn from_request(
        kinematics: &RobotKinematics,
        request: StepRequest,
        swing_side: Side,
    ) -> Self {
        let start = Feet::from_joints(kinematics, swing_side);

        let max_step = StepRequest::new(0.08, 0.08, 2.0);
        let step = request
            .clamp(-max_step, max_step)
            .clamp_anatomic(swing_side, 0.1);

        let end = Feet::from_request(step, swing_side);
        let duration = Duration::from_millis(250);

        Self {
            request: step,
            duration,
            start,
            end,
            foot_height: 0.012,
        }
    }
}
