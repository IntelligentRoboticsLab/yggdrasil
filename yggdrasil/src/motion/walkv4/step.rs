use std::time::Duration;

use super::Side;

#[derive(Debug, Clone, Copy)]
pub struct Step {
    pub forward: f32,
    pub left: f32,
    pub turn: f32,
    pub duration: Duration,
    pub swing_foot_height: f32,
    pub swing_foot: Side,
}

impl Default for Step {
    fn default() -> Self {
        Self {
            forward: 0f32,
            left: 0f32,
            turn: 0f32,
            duration: Duration::ZERO,
            swing_foot_height: 0f32,
            swing_foot: Side::default(),
        }
    }
}

impl Step {
    /// Clamps the step to the provided `max_step_size`.
    #[must_use]
    pub fn clamped(&self, max_step_size: Step) -> Step {
        Step {
            forward: self
                .forward
                .clamp(-max_step_size.forward, max_step_size.forward),
            left: self.left.clamp(-max_step_size.left, max_step_size.left),
            turn: self.turn.clamp(-max_step_size.turn, max_step_size.turn),
            ..*self
        }
    }

    /// Clamp the step to the anatomic limits of the robot.
    pub fn clamp_anatomic(self, max_inside_turn: f32) -> Self {
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
            ..self
        }
    }
}
