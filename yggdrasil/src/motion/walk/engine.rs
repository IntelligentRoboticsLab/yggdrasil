use nidhogg::types::{FillExt, Vector3};
use serde::{Deserialize, Serialize};

use crate::{
    kinematics::{forward::left_hip_to_ground, FootOffset, RobotKinematics},
    sensor::low_pass_filter::LowPassFilter,
};
use std::{ops::Neg, time::Duration};

use super::{smoothing, WalkingEngineConfig};

const FILTERED_GYRO_HIGH_PASS: f32 = 0.8;
const FILTERED_GYRO_LOW_PASS: f32 = 0.2;

#[derive(Debug, Clone)]
pub enum WalkState {
    /// Represents the robot in a standing position.
    ///
    /// The [`f32`] parameter specifies the desired hip height to smoothly transition the robot to an upright stance.
    Standing(f32),

    /// Represents the robot in a sitting position.
    ///
    /// The [`f32`] parameter specifies the desired hip height to smoothly transition the robot to an upright stance.
    Sitting(f32),

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
    /// Constructs an initial [`WalkState`] from a `hip_height`.
    ///
    /// This returns a new [`WalkState::Sitting`] using an estimated current `hip_height`.
    pub fn from_hip_height(hip_height: f32, config: &WalkingEngineConfig) -> Self {
        if hip_height <= config.sitting_hip_height {
            WalkState::Sitting(hip_height)
        } else {
            WalkState::Standing(hip_height)
        }
    }

    /// Transitions the [`WalkState`] to the next walk state based on the provided [`WalkingEngineConfig`].
    pub fn next(&self, config: &WalkingEngineConfig) -> Self {
        match self {
            WalkState::Standing(hip_height) => {
                WalkState::Standing((hip_height + 0.002).min(config.hip_height))
            }
            WalkState::Sitting(hip_height) => {
                WalkState::Sitting((hip_height - 0.002).max(config.sitting_hip_height))
            }
            WalkState::Starting(step) => WalkState::Walking(*step),
            WalkState::Walking(_) => self.clone(),
            WalkState::Stopping => WalkState::Standing(config.hip_height),
        }
    }
}

impl Default for WalkState {
    fn default() -> Self {
        WalkState::Standing(0.0)
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
    /// The filtered gyroscope values used for balancing.
    pub filtered_gyroscope: LowPassFilter<Vector3<f32>>,
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

impl Default for WalkingEngine {
    fn default() -> Self {
        WalkingEngine::new(&WalkingEngineConfig::default(), &RobotKinematics::default())
    }
}

impl WalkingEngine {
    /// Requests the [`WalkingEngine`] to halt to an idle sitting position.
    pub fn request_sit(&mut self) {
        self.request = WalkRequest::Sit;
    }

    /// Requests the [`WalkingEngine`] to go to an idle standing position.
    pub fn request_stand(&mut self) {
        self.request = WalkRequest::Stand;
    }

    /// Requests the [`WalkingEngine`] to perform the provided [`Step`].
    pub fn request_walk(&mut self, step: Step) {
        self.request = WalkRequest::Walk(step);
    }

    /// Returns whether the robot is currently sitting.
    ///
    /// TODO: Implement a better way to check if the robot is sitting, preferably by extracting sitting to a motion.
    pub fn is_sitting(&self) -> bool {
        matches!(self.state, WalkState::Sitting(hip_height) if hip_height <= self.config.sitting_hip_height)
    }

    /// Returns whether the robot is currently standing.
    pub fn is_standing(&self) -> bool {
        matches!(self.state, WalkState::Standing(hip_height) if hip_height >= self.config.hip_height)
    }

    /// Returns whether the robot is currently walking.
    pub fn is_walking(&self) -> bool {
        matches!(
            self.state,
            WalkState::Starting(_) | WalkState::Walking(_) | WalkState::Stopping
        )
    }

    pub(super) fn new(config: &WalkingEngineConfig, kinematics: &RobotKinematics) -> Self {
        let current_hip_height = left_hip_to_ground(kinematics);

        WalkingEngine {
            state: WalkState::from_hip_height(current_hip_height, config),
            request: WalkRequest::Sit,
            current_step: Step::default(),
            filtered_gyroscope: LowPassFilter::new(
                Vector3::default(),
                Vector3::fill(FILTERED_GYRO_HIGH_PASS),
                Vector3::fill(FILTERED_GYRO_LOW_PASS),
            ),
            t: Duration::ZERO,
            next_foot_switch: Duration::ZERO,
            swing_foot: Default::default(),
            foot_offsets: FootOffsets::zero(config.sitting_hip_height),
            foot_offsets_t0: FootOffsets::zero(config.sitting_hip_height),
            hip_height: current_hip_height,
            max_swing_foot_lift: config.base_foot_lift,
            config: config.clone(),
        }
    }

    /// Resets the properties of the walking engine, such that it results in a stationary upright position.
    pub(super) fn reset(&mut self) {
        self.current_step = Step::default();
        self.filtered_gyroscope = LowPassFilter::new(
            Vector3::default(),
            Vector3::fill(FILTERED_GYRO_HIGH_PASS),
            Vector3::fill(FILTERED_GYRO_LOW_PASS),
        );
        self.t = Duration::ZERO;
        self.foot_offsets = FootOffsets::zero(self.hip_height);
        self.foot_offsets_t0 = FootOffsets::zero(self.hip_height);
        self.swing_foot = Side::Left;
    }

    /// Initialises the next step phase.
    ///
    /// This will set the t0 foot offsets, and transition to the next [`WalkState`].
    /// It will then update the properties of the walking engine based on this new state.
    pub(super) fn init_step_phase(&mut self) {
        self.foot_offsets_t0 = self.foot_offsets.clone();
        let config = &self.config;
        self.state = self.state.next(config);

        match self.state {
            WalkState::Standing(hip_height) | WalkState::Sitting(hip_height) => {
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
                self.current_step = step.clamped(self.config.max_step_size);
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

        if let Some(new_state) = self.new_state_from_request() {
            self.state = new_state;
        }
    }

    fn new_state_from_request(&self) -> Option<WalkState> {
        match (&self.request, &self.state) {
            (WalkRequest::Sit, WalkState::Sitting(_) | WalkState::Stopping) => None,
            (WalkRequest::Stand, WalkState::Standing(_) | WalkState::Stopping) => None,
            // Stop walking
            (
                WalkRequest::Sit | WalkRequest::Stand,
                WalkState::Starting(_) | WalkState::Walking(_),
            ) => Some(WalkState::Stopping),
            // Start walking
            (WalkRequest::Walk(requested_step), WalkState::Standing(_) | WalkState::Stopping) => {
                if self.is_standing() {
                    Some(WalkState::Starting(*requested_step))
                } else {
                    None
                }
            }
            // Sit down
            (WalkRequest::Sit, WalkState::Standing(_)) => Some(WalkState::Sitting(self.hip_height)),
            // Stand up
            (WalkRequest::Stand | WalkRequest::Walk(_), WalkState::Sitting(_)) => {
                Some(WalkState::Standing(self.hip_height))
            }
            // Walking step change
            (WalkRequest::Walk(requested_step), state) => match state {
                WalkState::Starting(current_step) if current_step != requested_step => {
                    Some(WalkState::Starting(*requested_step))
                }
                WalkState::Walking(current_step) if current_step != requested_step => {
                    Some(WalkState::Walking(*requested_step))
                }
                _ => None,
            },
        }
    }

    /// Process the step phase.
    ///
    /// This increments the current phase time and computes the next foot offsets for the current step.
    pub(super) fn step_phase(&mut self, cycle_time: Duration) {
        self.t += cycle_time;
        self.foot_offsets = self.compute_foot_offsets(self.current_step);
    }

    /// End the current step phase.
    ///
    /// This will reset the current phase time to 0.
    pub fn end_step_phase(&mut self) {
        self.t = Duration::ZERO;
    }

    pub(super) fn compute_foot_offsets(&self, step: Step) -> FootOffsets {
        let linear_time =
            (self.t.as_secs_f32() / self.next_foot_switch.as_secs_f32()).clamp(0.0, 1.0);
        let swing_lift = self.max_swing_foot_lift * smoothing::parabolic_return(linear_time);

        let swing_foot = self.compute_swing_foot(step, swing_lift, linear_time);
        let support_foot = self.compute_support_foot(step, linear_time);

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

    fn compute_swing_foot(&self, step: Step, lift: f32, linear_time: f32) -> FootOffset {
        let smoothing = smoothing::parabolic_step(linear_time);
        let foot_t0 = match self.swing_foot {
            Side::Left => self.foot_offsets_t0.left,
            Side::Right => self.foot_offsets_t0.right,
        };
        self.compute_foot_offset(step, foot_t0, lift, true, smoothing)
    }

    fn compute_support_foot(&self, step: Step, linear_time: f32) -> FootOffset {
        let smoothing = linear_time;
        let foot_t0 = match self.swing_foot {
            Side::Left => self.foot_offsets_t0.right,
            Side::Right => self.foot_offsets_t0.left,
        };
        self.compute_foot_offset(-step, foot_t0, 0.0, false, smoothing)
    }

    fn compute_foot_offset(
        &self,
        step: Step,
        foot_t0: FootOffset,
        lift: f32,
        swing: bool,
        smoothing: f32,
    ) -> FootOffset {
        // The turn multiplier is divided by three, as the swing foot is active for two thirds of the step,
        // and the support foot is active for one third of the step.
        let turn_base = if swing { 2.0 } else { 1.0 };
        let turn_multiplier = match self.swing_foot {
            Side::Left => turn_base,
            Side::Right => -turn_base,
        } / 3.0;

        // The components are divided by two, as the step is split into two phases.
        FootOffset {
            forward: foot_t0.forward + (step.forward / 2.0 - foot_t0.forward) * smoothing,
            left: foot_t0.left + (step.left / 2.0 - foot_t0.left) * smoothing,
            turn: foot_t0.turn + (step.turn * turn_multiplier - foot_t0.turn) * smoothing,
            hip_height: self.hip_height,
            lift,
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Step {
    /// forward in meters per second
    pub forward: f32,
    /// side step in meters per second
    pub left: f32,
    /// turn in radians per second
    pub turn: f32,
}

impl Step {
    /// Clamps the step to the provided `max_step_size`.
    pub fn clamped(&self, max_step_size: Step) -> Step {
        Step {
            forward: self
                .forward
                .clamp(-max_step_size.forward, max_step_size.forward),
            left: self.left.clamp(-max_step_size.left, max_step_size.left),
            turn: self.turn.clamp(-max_step_size.turn, max_step_size.turn),
        }
    }
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

#[derive(Debug, Clone, PartialEq, Default)]
pub enum WalkRequest {
    Sit,
    #[default]
    Stand,
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
