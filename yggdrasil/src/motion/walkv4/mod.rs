use std::time::Duration;

use bevy::prelude::*;
use feet::FootPositions;
use nalgebra::{Isometry3, Point3, Translation, Translation3, UnitQuaternion, Vector3};
use nidhogg::types::{FillExt, LeftLegJoints, LegJoints, RightLegJoints};

use crate::{
    kinematics::{
        self,
        spaces::{
            LeftAnkle, LeftHip, LeftPelvis, LeftSole, LeftThigh, LeftTibia, RightHip, RightPelvis,
            RightSole, Robot,
        },
        FootOffset, Kinematics,
    },
    nao::{CycleTime, NaoManager, Priority},
    sensor::button::ChestButton,
};

use super::walk::WalkingEngineConfig;

mod feet;
mod step;

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
        app.init_resource::<WalkCommand>()
            .add_systems(Update, switch_phase)
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
    let (left, right) = kinematics::inverse::leg_angles(&foot_offset, &foot_offset);

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
    let (left, right) = kinematics::inverse::leg_angles(&foot_offset_left, &foot_offset);

    let leg_positions = LegJoints::builder().left_leg(left).right_leg(right).build();
    let leg_stiffness = LegJoints::builder()
        .left_leg(LeftLegJoints::fill(config.leg_stiffness))
        .right_leg(RightLegJoints::fill(config.leg_stiffness))
        .build();

    nao_manager.set_legs(leg_positions, leg_stiffness, Priority::Medium);
    *command = WalkCommand::Stand((hip_height + 0.0015).min(0.20));

    let robot_to_foot = Isometry3::from_parts(
        Translation3::new(0., -0.05, 0.225),
        UnitQuaternion::identity(),
    );

    let left_hip_to_ground = kinematics.isometry::<LeftAnkle, Robot>().inner.inverse();

    println!(
        "left_hip_to_robot: {:?}",
        kinematics
            .isometry::<LeftAnkle, Robot>()
            .inner
            .translation
            .vector
    );

    println!(
        "left_thigh_to_robot: {:?}",
        kinematics
            .isometry::<LeftThigh, Robot>()
            .inner
            .translation
            .vector
    );
    println!(
        "left_knee_to_robot: {:?}",
        kinematics
            .isometry::<LeftTibia, Robot>()
            .inner
            .translation
            .vector
    );
    println!(
        "left_ankle_to_robot: {:?}",
        kinematics
            .isometry::<LeftAnkle, Robot>()
            .inner
            .translation
            .vector
    );

    let zero_point =
        kinematics.isometry::<LeftAnkle, Robot>().inner.inverse() * Point3::new(0.0, 0.0, 0.0);
    tracing::info!(
        "left_foot_to_robot: {}",
        kinematics
            .isometry::<LeftSole, Robot>()
            .inner
            .translation
            .vector
    );

    let mut offset = kinematics
        .isometry::<LeftAnkle, Robot>()
        .inner
        .inverse()
        .translation
        .vector;
    offset.x = 0.0;
    offset.y = -0.05;

    // TODO: the torso offset is hard coded in the ik implementation!!
    let torso_offset = 0.025;
    let hip_height = 0.225;

    let foot_positions = FootPositions::from_kinematics(Side::Left, &kinematics, torso_offset);

    tracing::info!("feet: {:?}\n\n\n", foot_positions);
}

#[derive(Debug, Clone, Default)]
struct WalkState {
    phase: Duration,
    planned_duration: Duration,
    swing_foot: Side,
}

fn walk_phase(
    mut state: Local<WalkState>,
    mut command: ResMut<WalkCommand>,
    mut nao_manager: ResMut<NaoManager>,
    config: Res<WalkingEngineConfig>,
    kinematics: Res<Kinematics>,
    cycle_time: Res<CycleTime>,
) {
    let WalkCommand::Walk(hip_height) = *command else {
        return;
    };

    state.phase += cycle_time.duration;

    if state.phase.as_secs_f32() > 0.75 * state.planned_duration.as_secs_f32() {
        state.phase = Duration::ZERO;
        state.planned_duration = Duration::from_secs_f32(0.5);
        state.swing_foot = state.swing_foot.opposite();
    }

    let foot_offset = FootOffset {
        forward: 0.04,
        turn: 0.,
        hip_height,
        ..Default::default()
    };

    let (left, right) =
        kinematics::inverse::leg_angles(&foot_offset, &FootOffset::zero(hip_height));

    let leg_positions = LegJoints::builder().left_leg(left).right_leg(right).build();
    let leg_stiffness = LegJoints::builder()
        .left_leg(LeftLegJoints::fill(config.leg_stiffness))
        .right_leg(RightLegJoints::fill(config.leg_stiffness))
        .build();

    nao_manager.set_legs(leg_positions, leg_stiffness, Priority::Medium);
    *command = WalkCommand::Walk(hip_height);
}
