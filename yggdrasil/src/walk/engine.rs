use serde::{Deserialize, Serialize};

use crate::kinematics::FootOffset;
use std::{ops::Neg, time::Duration};

use super::{smoothing, WalkingEngineConfig};

#[derive(Debug, Clone)]
pub enum WalkState {
    /// Represents the robot in a standing position.
    ///
    /// The [`f32`] parameter specifies the hip height to smoothly transition the robot to an upright stance.
    Idle(f32),

    /// Initiates the walking phase.
    ///
    /// This phase starts by creating a rocking motion, lifting only the swing foot to prepare for a step.
    Starting(Step),

    /// Executes the walking action using the provided [`Step`].
    ///
    /// During this phase, the robot performs the provided step while alternating
    /// the swing foot to maintain balance.
    Walking(Step),

    /// Ends the walking phase.
    ///
    /// This state is used to halt the walking motion and bring the robot to a stop.
    Stopping,
}

impl WalkState {
    /// Constructs an initial [`WalkState`] from the provided [`WalkingEngineConfig`].
    ///
    /// This returns a new [`WalkState::Idle`] using the configured `sitting_hip_height` as initial
    /// hip height.
    pub fn from_config(config: &WalkingEngineConfig) -> Self {
        WalkState::Idle(config.sitting_hip_height)
    }

    /// Transitions the [`WalkState`] to the next walk state based on the provided [`WalkingEngineConfig`].
    pub fn next(&self, config: &WalkingEngineConfig) -> Self {
        match self {
            WalkState::Idle(hip_height) => {
                WalkState::Idle((*hip_height + 0.002).min(config.hip_height))
            }
            WalkState::Starting(step) => WalkState::Walking(*step),
            WalkState::Walking(_) => self.clone(),
            WalkState::Stopping => WalkState::Idle(config.hip_height),
        }
    }
}

/// An omni-directional humanoid gait generator based on Hengst, 2014
///
/// <https://cgi.cse.unsw.edu.au/~robocup/2014ChampionTeamPaperReports/20140930-Bernhard.Hengst-Walk2014Report.pdf>
#[derive(Debug)]
pub struct WalkingEngine {
    /// The current state of the walking engine
    pub state: WalkState,
    /// The current requested Walk Option
    pub request: WalkRequest,
    /// The step that the engine is currently performing.
    pub current_step: Step,
    /// The time into the current phase.
    pub t: Duration,
    /// The duration of the current step, e.g. the time until the next foot switch.
    pub next_foot_switch: Duration,
    /// The current swing foot of the robot.
    pub swing_foot: Side,
    /// The current foot offsets based on the requested step and the current phase of the walking engine.
    pub foot_offsets: FootOffsets,
    /// The foot offsets at the start of the current step phase.
    pub foot_offsets_t0: FootOffsets,
    /// The current hip height, relative to the ground in meters.
    pub hip_height: f32,
    /// The maximum distance the swing foot can be lifted off the ground in meters.
    pub max_swing_foot_lift: f32,
    /// The configuration of the walking engine.
    pub config: WalkingEngineConfig,
}

impl WalkingEngine {
    pub fn from_config(config: &WalkingEngineConfig) -> Self {
        tracing::info!("Using hip height: {}", config.hip_height);
        WalkingEngine {
            state: WalkState::from_config(config),
            request: WalkRequest::Idle,
            current_step: Step::default(),
            t: Duration::ZERO,
            next_foot_switch: Duration::ZERO,
            swing_foot: Default::default(),
            foot_offsets: FootOffsets::zero(config.sitting_hip_height),
            foot_offsets_t0: FootOffsets::zero(config.sitting_hip_height),
            hip_height: config.sitting_hip_height,
            max_swing_foot_lift: config.base_foot_lift,
            config: config.clone(),
        }
    }

    /// Resets the properties of the walking engine, such that it results in a stationary upright position.
    pub fn reset(&mut self) {
        self.current_step = Step::default();
        self.t = Duration::ZERO;
        self.foot_offsets = FootOffsets::zero(self.hip_height);
        self.foot_offsets_t0 = FootOffsets::zero(self.hip_height);
        self.swing_foot = Side::Left;
    }

    /// Initialises the next step phase.
    ///
    /// This will set the t0 foot offsets, and transition to the next [`WalkState`].
    /// It will then update the properties of the walking engine based on this new state.
    pub fn init_step_phase(&mut self) {
        let config = &self.config;
        self.foot_offsets_t0 = self.foot_offsets.clone();
        self.state = self.state.next(config);

        match self.state {
            WalkState::Idle(hip_height) => {
                self.current_step = Step::default();
                self.next_foot_switch = Duration::ZERO;
                self.swing_foot = Side::Left;
                self.max_swing_foot_lift = 0.0;
                self.hip_height = hip_height;
            }
            WalkState::Starting(_) => {
                self.current_step = Step::default();
                self.next_foot_switch = config.base_step_period;
                self.swing_foot = self.swing_foot.next();
            }
            WalkState::Walking(step) => {
                let next_swing_foot = self.swing_foot.next();

                // TODO: step duration increase?
                self.current_step = step;
                self.next_foot_switch = config.base_step_period;

                self.swing_foot = next_swing_foot;
                self.max_swing_foot_lift = config.base_foot_lift
                    + (step.forward.abs() * config.foot_lift_modifier.forward)
                    + (step.left.abs() * config.foot_lift_modifier.left);
            }
            WalkState::Stopping => {
                self.current_step = Step::default();
                self.next_foot_switch = config.base_step_period;
                self.swing_foot = self.swing_foot.next();
                self.max_swing_foot_lift = config.base_foot_lift;
            }
        }
    }

    /// Process the step phase.
    ///
    /// This increments the current phase time and computes the next foot offsets for the current step.
    pub fn step_phase(&mut self, cycle_time: Duration) {
        self.t += cycle_time;
        self.foot_offsets = self.compute_foot_offsets(self.current_step);
    }

    /// End the current step phase.
    ///
    /// This will reset the current phase time to 0.
    pub fn end_step_phase(&mut self) {
        self.t = Duration::ZERO;
    }

    pub fn compute_foot_offsets(&self, step: Step) -> FootOffsets {
        let linear_time =
            (self.t.as_secs_f32() / self.next_foot_switch.as_secs_f32()).clamp(0.0, 1.0);
        let parabolic_time = smoothing::parabolic_step(linear_time);

        let (swing_t0, support_t0) = match self.swing_foot {
            Side::Left => (self.foot_offsets_t0.left, self.foot_offsets_t0.right),
            Side::Right => (self.foot_offsets_t0.right, self.foot_offsets_t0.left),
        };
        let swing_lift = self.max_swing_foot_lift * smoothing::parabolic_return(linear_time);
        let support_lift = 0.0;

        let swing_foot = self.compute_foot_offset(step, swing_t0, swing_lift, 2.0, parabolic_time);
        let support_foot =
            self.compute_foot_offset(-step, support_t0, support_lift, 1.0, linear_time);
        match self.swing_foot {
            Side::Left => FootOffsets {
                left: swing_foot,
                right: support_foot,
            },
            Side::Right => FootOffsets {
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
        let turn_multiplier = match self.swing_foot {
            Side::Left => turn_base,
            Side::Right => -turn_base,
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

#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize)]
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
pub enum Side {
    #[default]
    Left,
    Right,
}

impl Side {
    pub fn next(&self) -> Self {
        match self {
            Side::Left => Side::Right,
            Side::Right => Side::Left,
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
