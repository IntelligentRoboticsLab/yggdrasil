use std::{
    ops::{Add, Neg, Sub},
    time::Duration,
};

use super::{feet::FootPositions, Side};

#[derive(Debug, Clone, Copy)]
pub struct Step {
    pub forward: f32,
    pub left: f32,
    pub turn: f32,
}

impl Step {
    pub fn clamp(&self, min: Step, max: Step) -> Self {
        Self {
            forward: self.forward.clamp(min.forward, max.forward),
            left: self.left.clamp(min.left, max.left),
            turn: self.turn.clamp(min.turn, max.turn),
        }
    }

    /// Clamp the step to the anatomic limits of the robot.
    pub fn clamp_anatomic(self, swing_foot: Side, max_inside_turn: f32) -> Self {
        let sideways_dir = if self.left.is_sign_positive() {
            Side::Left
        } else {
            Side::Right
        };

        let clamped_sideways = if sideways_dir == self.swing_foot {
            self.left
        } else {
            0.0
        };

        let turn_direction = if self.turn.is_sign_positive() {
            Side::Left
        } else {
            Side::Right
        };

        let clamped_turn = if turn_direction == self.swing_foot {
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

#[derive(Debug, Clone, Copy)]
pub struct PlannedStep {
    pub step: Step,
    pub start: FootPositions,
    pub end: FootPositions,
    pub duration: Duration,
    pub swing_foot_height: f32,
    pub swing_foot: Side,
}

impl PlannedStep {}
