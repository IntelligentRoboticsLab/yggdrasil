use std::time::Duration;

use nidhogg::{
    types::{FillExt, ForceSensitiveResistors, JointArray, LeftLegJoints, RightLegJoints},
    NaoControlMessage,
};

use crate::{
    debug::DebugContext,
    filter::button::{ChestButton, HeadButtons},
    kinematics::FootOffset,
    prelude::*,
    primary_state::PrimaryState,
};

use super::{smoothing, FilteredGyroscope, WalkingEngineConfig};
use crate::nao::CycleTime;

#[derive(Debug, Default, Clone, Copy)]
pub struct Step {
    /// forward in meters per second
    pub forward: f32,
    /// side step in meters per second
    pub left: f32,
    /// turn in radians per second
    pub turn: f32,
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

    pub swing_side: Side,
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
        self.swing_side = Side::Left;
    }

    pub fn init_step_phase(&mut self, config: &WalkingEngineConfig) {
        self.foot_offsets_t0 = self.foot_offsets.clone();
        self.state = self.state.next();

        // tracing::info!("init phase! {:?}", self.state);

        match self.state {
            WalkState::Idle => {
                self.current_step = Step::default();
                self.next_foot_switch = Duration::ZERO;
                self.swing_side = Side::Left;
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
        match self.swing_side {
            Side::Left => FootOffsets {
                left: self.compute_swing_foot(step, self.foot_offsets_t0.left),
                right: self.compute_support_foot(step, self.foot_offsets_t0.right),
            },
            Side::Right => FootOffsets {
                left: self.compute_support_foot(step, self.foot_offsets_t0.left),
                right: self.compute_swing_foot(step, self.foot_offsets_t0.right),
            },
        }
    }

    fn compute_support_foot(&self, step: Step, support_t0: FootOffset) -> FootOffset {
        let linear_time =
            (self.t.as_secs_f32() / self.next_foot_switch.as_secs_f32()).clamp(0.0, 1.0);

        let turn_multiplier = match self.swing_side {
            Side::Left => 2.0,
            Side::Right => -2.0,
        } / 3.0;

        FootOffset {
            forward: support_t0.forward
                + (-(step.forward) / 2.0 - support_t0.forward) * linear_time,
            left: support_t0.left + (-step.left / 2.0 - support_t0.left) * linear_time,
            turn: support_t0.turn + (-step.turn * turn_multiplier - support_t0.turn) * linear_time,
            hip_height: self.hip_height,
            lift: 0.0,
        }
    }

    fn compute_swing_foot(&self, step: Step, swing_t0: FootOffset) -> FootOffset {
        let linear_time =
            (self.t.as_secs_f32() / self.next_foot_switch.as_secs_f32()).clamp(0.0, 1.0);
        let parabolic_time = smoothing::parabolic_step(linear_time);

        let turn_multiplier = match self.swing_side {
            Side::Left => 2.0,
            Side::Right => -2.0,
        } / 3.0;

        FootOffset {
            forward: swing_t0.forward + (step.forward / 2.0 - swing_t0.forward) * parabolic_time,
            left: swing_t0.left + (step.left / 2.0 - swing_t0.left) * parabolic_time,
            turn: swing_t0.turn + (step.turn * turn_multiplier - swing_t0.turn) * parabolic_time,
            hip_height: self.hip_height,
            lift: self.max_foot_lift * smoothing::parabolic_return(linear_time),
        }
    }
}

#[system]
pub fn toggle_walking_engine(
    primary_state: &PrimaryState,
    head_button: &HeadButtons,
    chest_button: &ChestButton,
    walking_engine: &mut WalkingEngine,
    filtered_gyro: &mut FilteredGyroscope,
) -> Result<()> {
    // If we're in a state where we shouldn't walk, we don't.
    if !primary_state.should_walk() {
        return Ok(());
    }

    // Start walking
    if chest_button.state.is_tapped() {
        filtered_gyro.reset();
        walking_engine.state = WalkState::Starting(Step {
            forward: 0.06,
            left: 0.0,
            turn: 0.0,
        });
        return Ok(());
    }

    // Stop walking
    if head_button.front.is_tapped() {
        walking_engine.state = WalkState::Stopping;
        return Ok(());
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
#[system]
pub fn walking_engine(
    walking_engine: &mut WalkingEngine,
    config: &WalkingEngineConfig,
    primary_state: &PrimaryState,
    cycle_time: &CycleTime,
    fsr: &ForceSensitiveResistors,
    filtered_gyro: &FilteredGyroscope,
    control_message: &mut NaoControlMessage,
    dbg: &DebugContext,
) -> Result<()> {
    // We don't run the walking engine whenever we're in a state where we shouldn't.
    // This is a semi hacky way to prevent the robot from jumping up and
    // unstiffing itself when it's not supposed to.
    // TODO: We should definitely fix this in the future.deploy/assets deploy/config
    if !primary_state.should_walk() {
        return Ok(());
    }

    // If this is start of a new step phase, we'll need initialise the state.
    if walking_engine.t.is_zero() {
        walking_engine.init_step_phase(config)
    }

    match walking_engine.state {
        WalkState::Idle => walking_engine.reset(),
        WalkState::Starting(_) | WalkState::Walking(_) | WalkState::Stopping => {
            walking_engine.step_phase(cycle_time.duration);
        }
    }

    // check whether support foot has been switched
    let left_foot_fsr = fsr.left_foot.sum();
    let right_foot_fsr = fsr.right_foot.sum();

    let has_foot_switched = match walking_engine.swing_side {
        Side::Left => left_foot_fsr,
        Side::Right => right_foot_fsr,
    } > config.cop_pressure_threshold;

    let liner_time = (walking_engine.t.as_secs_f32()
        / walking_engine.next_foot_switch.as_secs_f32())
    .clamp(0.0, 1.0);

    if has_foot_switched && liner_time > 0.75 {
        walking_engine.end_step_phase();
    }

    let FootOffsets {
        left: left_foot,
        right: right_foot,
    } = walking_engine.foot_offsets.clone();

    // dbg.log_text("/foot/swing", format!("{:?}", walking_engine.swing_side))?;
    // dbg.log_scalar_f32("/foot/linear_time", liner_time)?;
    dbg.log_scalar_f32("/foot/left/forward", left_foot.forward)?;
    dbg.log_scalar_f32("/foot/left/lift", left_foot.lift)?;

    dbg.log_scalar_f32("/foot/right/forward", right_foot.forward)?;
    dbg.log_scalar_f32("/foot/right/lift", right_foot.lift)?;

    let (mut left_leg_joints, mut right_leg_joints) =
        crate::kinematics::inverse::leg_angles(&left_foot, &right_foot);

    // balancing

    // the shoulder pitch is "approximated" by taking the opposite direction multiplied by a constant.
    // this results in a left motion that moves in the opposite direction as the foot.
    let balancing_config = &config.balancing;
    let mut left_shoulder_pitch = -left_foot.forward * balancing_config.arm_swing_multiplier;
    let mut right_shoulder_pitch = -right_foot.forward * balancing_config.arm_swing_multiplier;

    // Balance adjustment
    let balance_adjustment = filtered_gyro.y() * balancing_config.filtered_gyro_y_multiplier;
    match walking_engine.swing_side {
        Side::Left => {
            right_leg_joints.ankle_pitch += balance_adjustment;
            left_shoulder_pitch = 0.0;
        }
        Side::Right => {
            left_leg_joints.ankle_pitch += balance_adjustment;
            right_shoulder_pitch = 0.0;
        }
    }

    control_message.position = JointArray::<f32>::builder()
        .left_shoulder_pitch(90f32.to_radians() + left_shoulder_pitch)
        .right_shoulder_pitch(90f32.to_radians() + right_shoulder_pitch)
        .left_leg_joints(left_leg_joints)
        .right_leg_joints(right_leg_joints)
        .build();

    let stiffness = 1.0;

    control_message.stiffness = JointArray::<f32>::builder()
        .left_shoulder_pitch(stiffness)
        .left_shoulder_roll(stiffness)
        .right_shoulder_pitch(stiffness)
        .right_shoulder_roll(stiffness)
        .head_pitch(stiffness)
        .head_yaw(stiffness)
        .left_leg_joints(LeftLegJoints::fill(stiffness))
        .right_leg_joints(RightLegJoints::fill(stiffness))
        .build();

    Ok(())
}
