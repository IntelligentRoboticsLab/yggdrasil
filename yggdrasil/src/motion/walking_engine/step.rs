use std::{
    ops::{Add, Div, Mul, Neg, Sub},
    time::Duration,
};

use serde::{Deserialize, Serialize};

use crate::kinematics::Kinematics;

use super::{feet::FootPositions, Side};

#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize)]
pub struct Step {
    /// Forward component of the step in meters.
    pub forward: f32,
    /// Sideways component of the step in meters.
    ///
    /// Positive values will result in a side step to the left, negative values will result
    /// in a side step to the right.
    pub left: f32,
    /// Turn component of the step in radians.
    ///
    /// Positive values will result in a turn to the left, and negative values will result
    /// in a turn to the right.
    pub turn: f32,
}

impl Step {
    #[must_use]
    pub fn clamp(&self, min: Step, max: Step) -> Self {
        Self {
            forward: self.forward.clamp(min.forward, max.forward),
            left: self.left.clamp(min.left, max.left),
            turn: self.turn.clamp(min.turn, max.turn),
        }
    }

    pub const FORWARD: Self = Self {
        forward: 0.06,
        left: 0.0,
        turn: 0.0,
    };

    pub const LEFT: Self = Self {
        forward: 0.0,
        left: 0.06,
        turn: 0.0,
    };

    pub const RIGHT: Self = Self {
        forward: 0.0,
        left: -0.06,
        turn: 0.0,
    };

    /// Clamp the step to the anatomic limits of the robot.
    #[must_use]
    pub fn clamp_anatomic(self, swing_foot: Side, max_inside_turn: f32) -> Self {
        let lateral_direction = if self.left.is_sign_positive() {
            Side::Left
        } else {
            Side::Right
        };

        let clamped_sideways = if lateral_direction == swing_foot {
            self.left
        } else {
            // lateral movement in the direction opposite to the current swing foot
            // would result in the robot hitting its ankles together, we avoid this
            // by clamping it to 0.
            0.0
        };

        let turn_direction = if self.turn.is_sign_positive() {
            Side::Left
        } else {
            Side::Right
        };

        let clamped_turn = if turn_direction == swing_foot {
            self.turn
        } else {
            // turning in the direction opposite to the current swing foot
            // makes it difficult for the robot to effectively turn, so we
            // clamp it to a small value to keep the motion
            self.turn.clamp(-max_inside_turn, max_inside_turn)
        };

        Self {
            forward: self.forward,
            left: clamped_sideways,
            turn: clamped_turn,
        }
    }
}

impl Add for Step {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            forward: self.forward + rhs.forward,
            left: self.left + rhs.left,
            turn: self.turn + rhs.turn,
        }
    }
}

impl Sub for Step {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            forward: self.forward - rhs.forward,
            left: self.left - rhs.left,
            turn: self.turn - rhs.turn,
        }
    }
}

impl Neg for Step {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self {
            forward: -self.forward,
            left: -self.left,
            turn: -self.turn,
        }
    }
}

impl Mul for Step {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self {
            forward: self.forward * rhs.forward,
            left: self.left * rhs.left,
            turn: self.turn * rhs.turn,
        }
    }
}

impl Div for Step {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        Self {
            forward: self.forward / rhs.forward,
            left: self.left / rhs.left,
            turn: self.turn / rhs.turn,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct PlannedStep {
    pub step: Step,
    pub start: FootPositions,
    pub target: FootPositions,
    pub duration: Duration,
    pub swing_foot_height: f32,
    pub swing_side: Side,
}

impl Default for PlannedStep {
    fn default() -> Self {
        Self {
            step: Step::default(),
            start: FootPositions::default(),
            target: FootPositions::default(),
            duration: Duration::from_millis(250),
            swing_foot_height: 0.,
            swing_side: Side::Left,
        }
    }
}

impl PlannedStep {
    /// Create a [`PlannedStep`] that is identical to [`PlannedStep::default`] but with
    /// the starting positions inferred from the kinematics.
    #[must_use]
    pub fn default_from_kinematics(kinematics: &Kinematics, torso_offset: f32) -> Self {
        Self {
            start: FootPositions::from_kinematics(Side::Left, kinematics, torso_offset),
            ..Default::default()
        }
    }
}
