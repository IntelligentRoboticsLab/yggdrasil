use crate::kinematics::FootOffset;
use std::{ops::Neg, time::Duration};

use super::{smoothing, WalkingEngineConfig};

#[derive(Debug, Default, Clone, Copy)]
pub struct Step {
    /// forward in meters per second
    pub forward: f32,
    /// side step in meters per second
    pub left: f32,
    /// turn in radians per second
    pub turn: f32,
}

impl Neg for Step {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Step {
            forward: -self.forward,
            left: -self.left,
            turn: -self.turn,
        }
    }
}

#[derive(Debug, Default, Clone)]
pub enum WalkRequest {
    #[default]
    Idle,
    Walk(Step),
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub enum SwingSide {
    #[default]
    Left,
    Right,
}

impl SwingSide {
    pub fn next(&self) -> Self {
        match self {
            SwingSide::Left => SwingSide::Right,
            SwingSide::Right => SwingSide::Left,
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct FootOffsets {
    pub left: FootOffset,
    pub right: FootOffset,
}

impl FootOffsets {
    pub fn zero(hip_height: f32) -> Self {
        FootOffsets {
            left: FootOffset::zero(hip_height),
            right: FootOffset::zero(hip_height),
        }
    }
}

#[derive(Debug, Default, Clone)]
pub enum WalkState {
    #[default]
    Idle,
    Starting(Step),
    Walking(Step),
    Stopping,
}

impl WalkState {
    pub fn next(&self) -> Self {
        match self {
            WalkState::Idle => WalkState::Idle,
            WalkState::Starting(step) => WalkState::Walking(*step),
            WalkState::Walking(_) => self.clone(),
            WalkState::Stopping => WalkState::Idle,
        }
    }
}

#[derive(Debug, Default)]
pub struct WalkingEngine {
    pub state: WalkState,
    pub request: WalkRequest,
    pub current_step: Step,
    pub t: Duration,
    pub next_foot_switch: Duration,

    pub swing_side: SwingSide,
    pub foot_offsets: FootOffsets,
    pub foot_offsets_t0: FootOffsets,

    pub hip_height: f32,
    pub max_foot_lift: f32,
}

impl WalkingEngine {
    pub fn new(config: &WalkingEngineConfig) -> Self {
        tracing::info!("Using hip height: {}", config.hip_height);
        WalkingEngine {
            hip_height: config.hip_height,
            ..Default::default()
        }
    }

    pub fn reset(&mut self) {
        self.current_step = Step::default();
        self.t = Duration::ZERO;
        self.foot_offsets = FootOffsets::zero(self.hip_height);
        self.foot_offsets_t0 = FootOffsets::zero(self.hip_height);
        self.swing_side = SwingSide::Left;
    }

    pub fn init_step_phase(&mut self, config: &WalkingEngineConfig) {
        self.foot_offsets_t0 = self.foot_offsets.clone();
        self.state = self.state.next();

        match self.state {
            WalkState::Idle => {
                self.current_step = Step::default();
                self.next_foot_switch = Duration::ZERO;
                self.swing_side = SwingSide::Left;
                self.max_foot_lift = 0.0;
            }
            WalkState::Starting(_) => {
                self.current_step = Step::default();
                self.next_foot_switch = config.base_step_period;
                self.swing_side = self.swing_side.next();
            }
            WalkState::Walking(step) => {
                let next_swing_foot = self.swing_side.next();

                // TODO: step duration increase?
                self.current_step = step;
                self.next_foot_switch = config.base_step_period;

                self.swing_side = next_swing_foot;
                self.max_foot_lift =
                    config.base_foot_lift + (step.forward.abs() * 0.01) + (step.left.abs() * 0.02);
            }
            WalkState::Stopping => {
                self.current_step = Step::default();
                self.next_foot_switch = config.base_step_period;
                self.swing_side = self.swing_side.next();
                self.max_foot_lift = config.base_foot_lift;
            }
        }
    }

    pub fn step_phase(&mut self, cycle_time: Duration) {
        self.t += cycle_time;
        self.foot_offsets = self.compute_foot_offsets(self.current_step);
    }

    pub fn end_step_phase(&mut self) {
        self.t = Duration::ZERO;
    }

    pub fn compute_foot_offsets(&self, step: Step) -> FootOffsets {
        let linear_time =
            (self.t.as_secs_f32() / self.next_foot_switch.as_secs_f32()).clamp(0.0, 1.0);
        let parabolic_time = smoothing::parabolic_step(linear_time);

        let (swing_t0, support_t0) = match self.swing_side {
            SwingSide::Left => (self.foot_offsets_t0.left, self.foot_offsets_t0.right),
            SwingSide::Right => (self.foot_offsets_t0.right, self.foot_offsets_t0.left),
        };
        let swing_lift = self.max_foot_lift * smoothing::parabolic_return(linear_time);
        let support_lift = 0.0;

        let swing_foot = self.compute_foot_offset(step, swing_t0, swing_lift, 2.0, parabolic_time);
        let support_foot =
            self.compute_foot_offset(-step, support_t0, support_lift, 1.0, linear_time);
        match self.swing_side {
            SwingSide::Left => FootOffsets {
                left: swing_foot,
                right: support_foot,
            },
            SwingSide::Right => FootOffsets {
                left: support_foot,
                right: swing_foot,
            },
        }
    }

    fn compute_foot_offset(
        &self,
        step: Step,
        foot_t0: FootOffset,
        lift: f32,
        turn_base: f32,
        smoothing: f32,
    ) -> FootOffset {
        let turn_multiplier = match self.swing_side {
            SwingSide::Left => turn_base,
            SwingSide::Right => -turn_base,
        } / 3.0;

        FootOffset {
            forward: foot_t0.forward + (step.forward / 2.0 - foot_t0.forward) * smoothing,
            left: foot_t0.left + (step.left / 2.0 - foot_t0.left) * smoothing,
            turn: foot_t0.turn + (step.turn * turn_multiplier - foot_t0.turn) * smoothing,
            hip_height: self.hip_height,
            lift,
        }
    }
}
