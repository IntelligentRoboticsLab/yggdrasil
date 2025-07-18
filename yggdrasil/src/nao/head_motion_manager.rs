use std::time::Instant;

use bevy::prelude::*;
use nalgebra::Point3;
use nidhogg::types::{FillExt, HeadJoints};

use crate::behavior::BehaviorConfig;
use crate::behavior::behaviors::ObserveBehaviorConfig;
use crate::localization::RobotPose;
use crate::nao::NaoManager;
use crate::nao::Priority;

pub(super) struct HeadMotionManagerPlugin;

impl Plugin for HeadMotionManagerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HeadMotionManager>()
            .add_systems(PostStartup, update_head_motion_config);
    }
}

fn update_head_motion_config(
    mut head_motion_manager: ResMut<HeadMotionManager>,
    behavior_config: Res<BehaviorConfig>,
) {
    head_motion_manager.update_config(behavior_config.observe.clone());
}

#[derive(Resource, Default)]
pub(crate) struct HeadMotionManager {
    observe_config: ObserveBehaviorConfig,
}

impl HeadMotionManager {
    fn update_config(&mut self, config: ObserveBehaviorConfig) {
        self.observe_config = config;
    }

    /// Set the robot head to a fixed position and stiffness
    pub(crate) fn fixed(
        &self,
        nao_manager: &mut NaoManager,
        yaw: f32,
        pitch: f32,
        stiffness: f32,
        priority: Priority,
    ) {
        let joint_positions = HeadJoints { yaw, pitch };
        let joint_stiffness = HeadJoints::fill(stiffness);
        update_head_motion(nao_manager, joint_positions, joint_stiffness, priority);
    }

    /// Head motion where the head will position it self to look at the given point
    pub(crate) fn look_at(
        &self,
        nao_manager: &mut NaoManager,
        point: &Point3<f32>,
        pose: &RobotPose,
        priority: Priority,
    ) {
        let joint_positions = pose.get_look_at_absolute(point);
        let joint_stiffness = HeadJoints::fill(self.observe_config.look_at_head_stiffness);

        update_head_motion(nao_manager, joint_positions, joint_stiffness, priority);
    }

    /// Set the robot head in a look around motion
    pub(crate) fn look_around(&self, nao_manager: &mut NaoManager, starting_time: Instant) {
        // Used to parameterize the yaw and pitch angles, multiplying with a large
        // rotation speed will make the rotation go faster.
        let movement_progress =
            starting_time.elapsed().as_secs_f32() * self.observe_config.head_rotation_speed;
        let yaw = (movement_progress).sin() * self.observe_config.head_yaw_max;
        let pitch = (movement_progress * 2.0 + std::f32::consts::FRAC_PI_2)
            .sin()
            .max(0.0)
            * self.observe_config.head_pitch_max;

        let joint_positions = HeadJoints { yaw, pitch };
        let joint_stiffness = HeadJoints::fill(self.observe_config.look_around_head_stiffness);

        update_head_motion(
            nao_manager,
            joint_positions,
            joint_stiffness,
            Priority::default(),
        );
    }
}

/// Update the head using the nao manager from head motion requests
fn update_head_motion(
    nao_manager: &mut NaoManager,
    joint_positions: HeadJoints<f32>,
    joint_sitffness: HeadJoints<f32>,
    priority: Priority,
) {
    nao_manager.set_head(joint_positions, joint_sitffness, priority);
}
