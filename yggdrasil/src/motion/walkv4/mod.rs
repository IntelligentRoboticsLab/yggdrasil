use bevy::prelude::*;
use nalgebra::{Isometry3, Point3, Quaternion, Translation3, UnitQuaternion, Vector3};
use nidhogg::types::{FillExt, LeftLegJoints, LegJoints, RightLegJoints};

use crate::{
    kinematics::{self, robot_dimensions, FootOffset, RobotKinematics},
    nao::{NaoManager, Priority},
    sensor::button::ChestButton,
};

use super::walk::WalkingEngineConfig;

mod step;

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    #[default]
    Left,
    Right,
}

pub struct Walkv4EnginePlugin;

impl Plugin for Walkv4EnginePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WalkCommand>()
            .add_systems(Update, switch_phase)
            .add_systems(PostUpdate, (sit_phase, stand_phase));
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Resource)]
enum WalkCommand {
    Sit(f32),
    Stand(f32),
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
            WalkCommand::Stand(hip_height) => WalkCommand::Sit(hip_height),
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

struct FootPositions {
    left: Isometry3<f32>,
    right: Isometry3<f32>,
}

fn stand_phase(
    mut command: ResMut<WalkCommand>,
    mut nao_manager: ResMut<NaoManager>,
    config: Res<WalkingEngineConfig>,
    kinematics: Res<RobotKinematics>,
) {
    let WalkCommand::Stand(hip_height) = *command else {
        return;
    };

    let foot_offset_left = FootOffset {
        forward: 0.04,
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
    *command = WalkCommand::Stand((hip_height + 0.0015).min(0.18));

    let robot_to_foot = Isometry3::from_parts(
        Translation3::new(0., -0.05, 0.225),
        UnitQuaternion::identity(),
    );
    let left_hip_to_ground = kinematics.left_ankle_to_robot.inverse();

    println!(
        "left_hip_to_robot: {:?}",
        kinematics.left_hip_to_robot.translation.vector
    );

    println!(
        "left_thing_to_robot: {:?}",
        kinematics.left_thigh_to_robot.translation.vector
    );
    println!(
        "left_knee_to_robot: {:?}",
        kinematics.left_tibia_to_robot.translation.vector
    );
    println!(
        "left_ankle_to_robot: {:?}",
        kinematics.left_ankle_to_robot.translation.vector
    );

    let zero_point = kinematics.left_ankle_to_robot.inverse() * Point3::new(0.0, 0.0, 0.0);
    tracing::info!(
        "left_foot_to_robot: {}",
        kinematics.left_sole_to_robot.translation.vector
    );

    let mut offset = kinematics.left_ankle_to_robot.inverse().translation.vector;
    offset.x = 0.0;
    offset.y = -0.05;

    // TODO: the torso offset is hard coded in the ik implementation!!
    let torso_offset = 0.025;
    let foot_position =
        (kinematics.left_sole_to_robot).translation.vector + Vector3::new(torso_offset, 0.0, 0.225);

    tracing::info!("left_foot: {}", foot_position);
}
