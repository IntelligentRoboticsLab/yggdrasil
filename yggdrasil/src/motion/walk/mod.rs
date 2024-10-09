pub mod engine;
pub mod smoothing;
use std::time::Duration;

use crate::{
    core::config::ConfigExt,
    kinematics::RobotKinematics,
    nao::{CycleTime, NaoManager, Priority},
    prelude::*,
    sensor::imu::IMUValues,
};
use bevy::prelude::*;
use nidhogg::{
    types::{
        ArmJoints, FillExt, ForceSensitiveResistors, LeftArmJoints, LeftLegJoints, LegJoints,
        RightArmJoints, RightLegJoints,
    },
    NaoState,
};
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DurationMilliSeconds};

use self::engine::{FootOffsets, Side, Step, WalkState, WalkingEngine};

#[derive(Debug, Default, Clone, Resource)]
pub struct SwingFoot {
    side: Side,
}

impl SwingFoot {
    #[must_use]
    pub fn support(&self) -> Side {
        match self.side {
            Side::Left => Side::Right,
            Side::Right => Side::Left,
        }
    }

    #[must_use]
    pub fn swing(&self) -> Side {
        self.side
    }
}

#[derive(Resource, Serialize, Deserialize, Debug, Clone, Default)]
pub struct BalancingConfig {
    pub arm_swing_multiplier: f32,
    pub filtered_gyro_y_multiplier: f32,
}

/// Configuration for the walking engine.
#[serde_as]
#[derive(Resource, Serialize, Deserialize, Debug, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct WalkingEngineConfig {
    #[serde_as(as = "DurationMilliSeconds")]
    pub base_step_period: Duration,
    pub leg_stiffness: f32,
    pub arm_stiffness: f32,
    pub cop_pressure_threshold: f32,
    pub base_foot_lift: f32,
    pub foot_lift_modifier: Step,
    pub max_step_size: Step,
    pub hip_height: f32,
    pub sitting_hip_height: f32,
    pub balancing: BalancingConfig,
}

impl Config for WalkingEngineConfig {
    const PATH: &'static str = "walking_engine.toml";
}

/// Plugin providing systems the walking engine for the robot.
pub struct WalkingEnginePlugin;

impl Plugin for WalkingEnginePlugin {
    fn build(&self, app: &mut App) {
        app.init_config::<WalkingEngineConfig>()
            .init_resource::<SwingFoot>();

        app.add_systems(PostStartup, init_walking_engine);
        app.add_systems(PostUpdate, (run_walking_engine, update_swing_side).chain());
    }
}

fn init_walking_engine(
    mut commands: Commands,
    config: Res<WalkingEngineConfig>,
    nao_state: Res<NaoState>,
) {
    let kinematics = RobotKinematics::from(&nao_state.position);

    commands.insert_resource(WalkingEngine::new(&config, &kinematics));
}

/// System that executes the walking engine.
fn run_walking_engine(
    mut walking_engine: ResMut<WalkingEngine>,
    cycle_time: Res<CycleTime>,
    fsr: Res<ForceSensitiveResistors>,
    imu: Res<IMUValues>,
    mut nao_manager: ResMut<NaoManager>,
) {
    // If this is start of a new step phase, we'll need to initialise the new phase.
    if walking_engine.t.is_zero() {
        walking_engine.init_step_phase();
    }

    match walking_engine.state {
        WalkState::Sitting(_) | WalkState::Standing(_) => walking_engine.reset(),
        WalkState::Starting(_) | WalkState::Walking(_) | WalkState::Stopping => {
            walking_engine.step_phase(cycle_time.duration);
        }
    }

    // check whether support foot has been switched
    let config = walking_engine.config.clone();
    let left_foot_fsr = fsr.left_foot.sum();
    let right_foot_fsr = fsr.right_foot.sum();

    let has_foot_switched = match walking_engine.swing_foot {
        Side::Left => left_foot_fsr,
        Side::Right => right_foot_fsr,
    } > config.cop_pressure_threshold;

    let linear_time = (walking_engine.t.as_secs_f32()
        / walking_engine.next_foot_switch.as_secs_f32())
    .clamp(0.0, 1.0);

    if has_foot_switched && linear_time > 0.75 {
        walking_engine.end_step_phase();
    }

    let FootOffsets {
        left: left_foot,
        right: right_foot,
    } = walking_engine.foot_offsets.clone();

    let (mut left_leg_joints, mut right_leg_joints) =
        crate::kinematics::inverse::leg_angles(&left_foot, &right_foot);

    // TODO: proper balancing

    // the shoulder pitch is "approximated" by taking the opposite direction multiplied by a constant.
    // this results in a left motion that moves in the opposite direction as the foot.
    let balancing_config = &config.balancing;
    let mut left_shoulder_pitch = -left_foot.forward * balancing_config.arm_swing_multiplier;
    let mut right_shoulder_pitch = -right_foot.forward * balancing_config.arm_swing_multiplier;

    // Balance adjustment
    walking_engine.filtered_gyroscope.update(imu.gyroscope);
    let balance_adjustment =
        walking_engine.filtered_gyroscope.state.y * balancing_config.filtered_gyro_y_multiplier;
    match walking_engine.swing_foot {
        Side::Left => {
            right_leg_joints.ankle_pitch += balance_adjustment;
            left_shoulder_pitch = 0.0;
        }
        Side::Right => {
            left_leg_joints.ankle_pitch += balance_adjustment;
            right_shoulder_pitch = 0.0;
        }
    }

    let leg_positions = LegJoints::builder()
        .left_leg(left_leg_joints)
        .right_leg(right_leg_joints)
        .build();
    let leg_stiffness = LegJoints::builder()
        .left_leg(LeftLegJoints::fill(config.leg_stiffness))
        .right_leg(RightLegJoints::fill(config.leg_stiffness))
        .build();

    let arm_positions = ArmJoints::builder()
        .left_arm(
            LeftArmJoints::builder()
                .shoulder_pitch(90f32.to_radians() + left_shoulder_pitch)
                .build(),
        )
        .right_arm(
            RightArmJoints::builder()
                .shoulder_pitch(90f32.to_radians() + right_shoulder_pitch)
                .build(),
        )
        .build();
    let arm_stiffness = ArmJoints::builder()
        .left_arm(
            LeftArmJoints::builder()
                .shoulder_pitch(config.arm_stiffness)
                .build(),
        )
        .right_arm(
            RightArmJoints::builder()
                .shoulder_pitch(config.arm_stiffness)
                .build(),
        )
        .build();

    nao_manager
        .set_legs(leg_positions, leg_stiffness, Priority::Medium)
        .set_arms(arm_positions, arm_stiffness, Priority::Medium);
}

fn update_swing_side(walking_engine: Res<WalkingEngine>, mut swing_foot: ResMut<SwingFoot>) {
    swing_foot.side = walking_engine.swing_foot;
}
