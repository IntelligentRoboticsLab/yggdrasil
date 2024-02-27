pub mod engine;
pub mod smoothing;
use std::time::Duration;

use crate::{
    debug::DebugContext,
    filter::button::{ChestButton, HeadButtons},
    nao::CycleTime,
    prelude::*,
    primary_state::PrimaryState,
};
use nidhogg::{
    types::{
        FillExt, ForceSensitiveResistors, JointArray, LeftLegJoints, RightLegJoints, Vector2,
        Vector3,
    },
    NaoControlMessage,
};
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DurationMilliSeconds};

use crate::{filter, nao, primary_state};

use self::engine::{FootOffsets, Side, Step, WalkState, WalkingEngine};

#[derive(Debug, Default, Clone)]
pub struct SwingFoot {
    pub side: Side,
}

/// Filtered gyroscope values.
#[derive(Default, Debug, Clone)]
pub struct FilteredGyroscope(Vector2<f32>);

impl FilteredGyroscope {
    pub fn update(&mut self, gyroscope: &Vector3<f32>) {
        self.0.x = 0.8 * self.0.x + 0.2 * gyroscope.x;
        self.0.y = 0.8 * self.0.y + 0.2 * gyroscope.y;
    }

    pub fn reset(&mut self) {
        self.0 = Vector2::default();
    }

    pub fn x(&self) -> f32 {
        self.0.x
    }

    pub fn y(&self) -> f32 {
        self.0.y
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct BalancingConfig {
    pub arm_swing_multiplier: f32,
    pub filtered_gyro_y_multiplier: f32,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct WalkingEngineConfig {
    #[serde_as(as = "DurationMilliSeconds")]
    pub base_step_period: Duration,
    pub cop_pressure_threshold: f32,
    pub base_foot_lift: f32,
    pub hip_height: f32,
    pub sitting_hip_height: f32,
    pub balancing: BalancingConfig,
}

impl Config for WalkingEngineConfig {
    const PATH: &'static str = "walking_engine.toml";
}

/// A module providing the walking engine for the robot.
///
/// This module provides the following resources to the application:
/// - [`WalkingEngine`]
/// - [`FilteredGyroscope`]
pub struct WalkingEngineModule;

impl Module for WalkingEngineModule {
    fn initialize(self, app: App) -> Result<App> {
        Ok(app
            .init_config::<WalkingEngineConfig>()?
            .init_resource::<FilteredGyroscope>()?
            .init_resource::<SwingFoot>()?
            .add_startup_system(init_walking_engine)?
            // .add_system_chain((
            //     (nao::write_hardware_info, nao::update_cycle_stats),
            //     (
            //         filter::button::button_filter,
            //         filter::fsr::force_sensitive_resistor_filter,
            //         filter::imu::imu_filter,
            //     ),
            //     (
            //         filter_gyro_values,
            //         toggle_walking_engine,
            //         run_walking_engine,
            //         update_swing_side,
            //         primary_state::update_primary_state,
            //     ),
            // )))
            .add_system(
                filter_gyro_values
                    .after(nao::write_hardware_info)
                    .after(filter::imu::imu_filter),
            )
            .add_system(
                run_walking_engine
                    .before(primary_state::update_primary_state)
                    .after(nao::update_cycle_stats)
                    .after(filter_gyro_values)
                    .after(filter::fsr::force_sensitive_resistor_filter),
            )
            .add_system(
                toggle_walking_engine
                    .before(primary_state::update_primary_state)
                    .after(filter::button::button_filter)
                    .before(run_walking_engine),
            )
            .add_system(update_swing_side.after(run_walking_engine)))
    }
}

#[startup_system]
fn init_walking_engine(storage: &mut Storage, config: &WalkingEngineConfig) -> Result<()> {
    storage.add_resource(Resource::new(WalkingEngine::new(config)))
}

#[system]
fn filter_gyro_values(
    imu_values: &filter::imu::IMUValues,
    filtered_gyro: &mut FilteredGyroscope,
) -> Result<()> {
    filtered_gyro.update(&imu_values.gyroscope);

    Ok(())
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
            forward: 0.05,
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
pub fn run_walking_engine(
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

    // If this is start of a new step phase, we'll need to initialise the new phase.
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

    dbg.log_scalar_f32("/foot/left/forward", left_foot.forward)?;
    dbg.log_scalar_f32("/foot/left/left", left_foot.left)?;
    dbg.log_scalar_f32("/foot/left/turn", left_foot.turn)?;
    dbg.log_scalar_f32("/foot/left/lift", left_foot.lift)?;

    dbg.log_scalar_f32("/foot/right/forward", right_foot.forward)?;
    dbg.log_scalar_f32("/foot/right/left", right_foot.left)?;
    dbg.log_scalar_f32("/foot/right/turn", right_foot.turn)?;
    dbg.log_scalar_f32("/foot/right/lift", right_foot.lift)?;

    let (mut left_leg_joints, mut right_leg_joints) =
        crate::kinematics::inverse::leg_angles(&left_foot, &right_foot);

    // TODO: proper balancing

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
        .right_shoulder_pitch(stiffness)
        .head_pitch(stiffness)
        .head_yaw(stiffness)
        .left_leg_joints(LeftLegJoints::fill(stiffness))
        .right_leg_joints(RightLegJoints::fill(stiffness))
        .build();

    Ok(())
}

#[system]
fn update_swing_side(walking_engine: &WalkingEngine, swing_foot: &mut SwingFoot) -> Result<()> {
    swing_foot.side = walking_engine.swing_side;
    Ok(())
}
