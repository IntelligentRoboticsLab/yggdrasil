use std::time::Duration;

use bevy::prelude::*;
use feet::FootPositions;
use nalgebra::{Isometry3, Point3, Translation, Translation3, UnitQuaternion, Vector3};
use nidhogg::{
    types::{FillExt, ForceSensitiveResistors, LeftLegJoints, LegJoints, RightLegJoints},
    NaoState,
};
use step::Step;

use crate::{
    core::debug::DebugContext,
    kinematics::{
        self,
        prelude::{ROBOT_TO_LEFT_PELVIS, ROBOT_TO_RIGHT_PELVIS},
        spaces::{
            LeftAnkle, LeftHip, LeftPelvis, LeftSole, LeftThigh, LeftTibia, RightHip, RightPelvis,
            RightSole, Robot,
        },
        FootOffset, Kinematics,
    },
    motion::walk::smoothing::{parabolic_return, parabolic_step},
    nao::{Cycle, CycleTime, NaoManager, Priority},
    sensor::{
        button::{ChestButton, HeadButtons},
        fsr::Contacts,
        imu::IMUValues,
        low_pass_filter::LowPassFilter,
        orientation::RobotOrientation,
    },
};

use super::walk::WalkingEngineConfig;

mod feet;
mod gait;
mod scheduling;
mod step;
mod support_foot;

const TORSO_OFFSET: f32 = 0.025;

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    #[default]
    Left,
    Right,
}

impl Side {
    #[must_use]
    pub fn opposite(self) -> Self {
        match self {
            Side::Left => Side::Right,
            Side::Right => Side::Left,
        }
    }
}

pub struct Walkv4EnginePlugin;

impl Plugin for Walkv4EnginePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(support_foot::SupportFootPlugin);
        app.init_resource::<WalkCommand>()
            .add_systems(Update, (switch_phase, switch_to_sitting))
            .add_systems(PostUpdate, (sit_phase, stand_phase, walk_phase));
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Resource)]
enum WalkCommand {
    Sit(f32),
    Stand(f32),
    Walk(f32),
}

impl Default for WalkCommand {
    fn default() -> Self {
        Self::Sit(0.094)
    }
}

fn switch_phase(mut command: ResMut<WalkCommand>, button: Res<ChestButton>) {
    if button.state.is_tapped() {
        *command = match *command {
            WalkCommand::Sit(hip_height) => WalkCommand::Stand(hip_height),
            WalkCommand::Stand(hip_height) => WalkCommand::Walk(hip_height),
            WalkCommand::Walk(hip_height) => WalkCommand::Stand(hip_height),
        };
    }
}

fn sit_phase(
    mut command: ResMut<WalkCommand>,
    mut nao_manager: ResMut<NaoManager>,
    config: Res<WalkingEngineConfig>,
) {
    let WalkCommand::Sit(hip_height) = *command else {
        return;
    };

    let foot_offset = FootOffset::zero(hip_height);
    let (left, right) = kinematics::inverse::leg_angles(&foot_offset, &foot_offset, TORSO_OFFSET);

    let leg_positions = LegJoints::builder().left_leg(left).right_leg(right).build();
    let leg_stiffness = LegJoints::builder()
        .left_leg(LeftLegJoints::fill(config.leg_stiffness))
        .right_leg(RightLegJoints::fill(config.leg_stiffness))
        .build();

    nao_manager.set_legs(leg_positions, leg_stiffness, Priority::Medium);
    *command = WalkCommand::Sit((hip_height - 0.001).max(0.094));
}

fn stand_phase(
    mut command: ResMut<WalkCommand>,
    mut nao_manager: ResMut<NaoManager>,
    config: Res<WalkingEngineConfig>,
    kinematics: Res<Kinematics>,
) {
    let WalkCommand::Stand(hip_height) = *command else {
        return;
    };

    let foot_offset_left = FootOffset {
        forward: 0.0,
        turn: 0.,
        hip_height,
        ..Default::default()
    };
    let foot_offset = FootOffset::zero(hip_height);
    let (left, right) =
        kinematics::inverse::leg_angles(&foot_offset_left, &foot_offset, TORSO_OFFSET);

    let leg_positions = LegJoints::builder().left_leg(left).right_leg(right).build();
    let leg_stiffness = LegJoints::builder()
        .left_leg(LeftLegJoints::fill(config.leg_stiffness))
        .right_leg(RightLegJoints::fill(config.leg_stiffness))
        .build();

    nao_manager.set_legs(leg_positions, leg_stiffness, Priority::Medium);
    *command = WalkCommand::Stand((hip_height + 0.0015).min(0.18));

    let robot_to_foot = Isometry3::from_parts(
        Translation3::new(0., -0.05, 0.225),
        UnitQuaternion::identity(),
    );

    let mut offset = kinematics
        .isometry::<LeftAnkle, Robot>()
        .inner
        .inverse()
        .translation
        .vector;
    offset.x = 0.0;
    offset.y = -0.05;

    let hip_height = 0.225;

    // TODO: the torso offset is hard coded in the ik implementation!!
    let foot_positions = FootPositions::from_kinematics(Side::Left, &kinematics, TORSO_OFFSET);

    // tracing::info!("feet: {:?}\n\n\n", foot_positions);
}

#[derive(Debug, Clone)]
struct WalkState {
    phase: Duration,
    start: FootPositions,
    planned_duration: Duration,
    swing_foot: Side,
    filtered_gyro: LowPassFilter<3>,
}

impl Default for WalkState {
    fn default() -> Self {
        Self {
            phase: Duration::ZERO,
            start: FootPositions::default(),
            planned_duration: Duration::from_secs_f32(0.25),
            swing_foot: Side::Left,
            filtered_gyro: LowPassFilter::new(0.115),
        }
    }
}

fn switch_to_sitting(head_buttons: Res<HeadButtons>, mut command: ResMut<WalkCommand>) {
    if head_buttons.all_pressed() {
        *command = WalkCommand::Sit(0.18);
    }
}

fn walk_phase(
    dbg: DebugContext,
    mut walk_state: Local<WalkState>,
    nao_state: Res<NaoState>,
    mut command: ResMut<WalkCommand>,
    mut nao_manager: ResMut<NaoManager>,
    config: Res<WalkingEngineConfig>,
    kinematics: Res<Kinematics>,
    cycle: Res<Cycle>,
    cycle_time: Res<CycleTime>,
    fsr: Res<ForceSensitiveResistors>,
    imu: Res<IMUValues>,
) {
    let WalkCommand::Walk(hip_height) = *command else {
        return;
    };

    walk_state.phase += cycle_time.duration;
    walk_state.filtered_gyro.update(imu.gyroscope);

    dbg.log_with_cycle(
        "gyro/filtered_y",
        *cycle,
        &rerun::Scalar::new(walk_state.filtered_gyro.state().y as f64),
    );

    let config = config.clone();
    let left_foot_fsr = fsr.left_foot.sum();
    let right_foot_fsr = fsr.right_foot.sum();

    let has_foot_switched = match walk_state.swing_foot {
        Side::Left => left_foot_fsr,
        Side::Right => right_foot_fsr,
    } > config.cop_pressure_threshold;

    let linear = (walk_state.phase.as_secs_f32() / walk_state.planned_duration.as_secs_f32())
        .clamp(0.0, 1.0);
    let parabolic = parabolic_step(linear);

    let step = Step::new(
        0.05,
        0.0,
        0.0,
        walk_state.planned_duration,
        0.01,
        walk_state.swing_foot,
    );

    let target = FootPositions::from_target(&step);

    let (left_t, right_t) = match &step.swing_foot {
        Side::Left => (parabolic, linear),
        Side::Right => (linear, parabolic),
    };

    let left = walk_state.start.left.lerp_slerp(&target.left.inner, left_t);
    let right = walk_state
        .start
        .right
        .lerp_slerp(&target.right.inner, right_t);

    let swing_lift = parabolic_return(linear) * 0.012;
    let (left_lift, right_lift) = match &step.swing_foot {
        Side::Left => (swing_lift, 0.),
        Side::Right => (0., swing_lift),
    };

    // println!("left: {:?}", left.translation);
    // println!("right: {:?}\n\n\n", right.translation);

    let current =
        FootPositions::from_kinematics(walk_state.swing_foot.opposite(), &kinematics, TORSO_OFFSET);

    dbg.log_with_cycle(
        "walk/swing_foot",
        *cycle,
        &rerun::Scalar::new(match walk_state.swing_foot {
            Side::Left => 0.06,
            Side::Right => -0.06,
        }),
    );

    dbg.log_with_cycle(
        "walk/left_lift_gt",
        *cycle,
        &rerun::Scalar::new(current.left.translation.z as f64),
    );

    dbg.log_with_cycle(
        "walk/right_lift_gt",
        *cycle,
        &rerun::Scalar::new(current.right.translation.z as f64),
    );

    dbg.log_with_cycle(
        "walk/left_lift",
        *cycle,
        &rerun::Scalar::new(left_lift as f64),
    );

    dbg.log_with_cycle(
        "walk/right_lift",
        *cycle,
        &rerun::Scalar::new(right_lift as f64),
    );

    dbg.log_with_cycle(
        "walk/left_forward_gt",
        *cycle,
        &rerun::Scalar::new(current.left.translation.x as f64),
    );

    dbg.log_with_cycle(
        "walk/right_forward_gt",
        *cycle,
        &rerun::Scalar::new(current.right.translation.x as f64),
    );

    dbg.log_with_cycle(
        "walk/left_forward",
        *cycle,
        &rerun::Scalar::new(left.translation.x as f64),
    );
    dbg.log_with_cycle(
        "walk/right_forward",
        *cycle,
        &rerun::Scalar::new(right.translation.x as f64),
    );

    // info!("state phase: {:?}", state.phase.as_secs_f32());

    let left_foot_offset = FootOffset {
        forward: left.translation.x,
        left: left.translation.y - ROBOT_TO_LEFT_PELVIS.y,
        turn: 0.,
        lift: left_lift,
        hip_height,
        ..Default::default()
    };

    let right_foot_offset = FootOffset {
        forward: right.translation.x,
        left: right.translation.y - ROBOT_TO_RIGHT_PELVIS.y,
        turn: 0.,
        lift: right_lift,
        hip_height,
        ..Default::default()
    };

    let (mut left, mut right) =
        kinematics::inverse::leg_angles(&left_foot_offset, &right_foot_offset, TORSO_OFFSET);

    if has_foot_switched && linear > 0.95 {
        walk_state.phase = Duration::ZERO;
        walk_state.planned_duration = Duration::from_secs_f32(0.25);
        walk_state.start = FootPositions::from_kinematics(
            walk_state.swing_foot.opposite(),
            &kinematics,
            TORSO_OFFSET,
        );
        walk_state.swing_foot = walk_state.swing_foot.opposite();
    }

    // Balance adjustment
    let balance_adjustment =
        walk_state.filtered_gyro.state().y * config.balancing.filtered_gyro_y_multiplier;
    match walk_state.swing_foot {
        Side::Left => {
            right.ankle_pitch += balance_adjustment;
        }
        Side::Right => {
            left.ankle_pitch += balance_adjustment;
        }
    }

    dbg.log_with_cycle(
        "walk/balance_adjustment",
        *cycle,
        &rerun::Scalar::new(balance_adjustment as f64),
    );

    let leg_positions = LegJoints::builder().left_leg(left).right_leg(right).build();
    let leg_stiffness = LegJoints::builder()
        .left_leg(LeftLegJoints::fill(config.leg_stiffness))
        .right_leg(RightLegJoints::fill(config.leg_stiffness))
        .build();

    nao_manager.set_legs(leg_positions, leg_stiffness, Priority::Medium);
    *command = WalkCommand::Walk(hip_height);
}
