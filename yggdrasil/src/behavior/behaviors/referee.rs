use std::time::Duration;

use bevy::prelude::*;
use nalgebra::Point3;

use crate::{
    behavior::engine::{in_behavior, Behavior, BehaviorState}, core::config::layout::LayoutConfig, localization::RobotPose, nao::{NaoManager, Priority}, vision::referee::{DetectRefereePose, VisualRefereeDetectionStatus}
};

const REFEREE_AVG_HEIGHT: f32 = 1.62;
const HEAD_ROTATION_TIME: Duration = Duration::from_millis(500);

pub struct VisualRefereeBehaviorPlugin;

impl Plugin for VisualRefereeBehaviorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            detect_visual_referee.run_if(in_behavior::<VisualReferee>.and(in_state(VisualRefereeDetectionStatus::Inactive))),
        );
    }
}

#[derive(Resource)]
pub struct VisualReferee;

impl Behavior for VisualReferee {
    const STATE: BehaviorState = BehaviorState::VisualReferee;
}

fn detect_visual_referee(
    // mut commands: Commands,
    layout_config: Res<LayoutConfig>,
    robot_pose: Res<RobotPose>,
    mut nao_manager: ResMut<NaoManager>,
    // mut step_context: ResMut<StepContext>,
    mut detect_pose: EventWriter<DetectRefereePose>,
) {
    // Make the robot look at the opposite T junction (relative to the starting)
    // position.
    let field_y_max = layout_config.field.width / 2.;
    let world_position = robot_pose.world_position();

    // commands.set_behavior(StandLookAt {
    //     target: Point2::new(0., -field_y_max * world_position.y.signum()),
    // });

    let point3 = Point3::new(
        0.,
        -field_y_max * world_position.y.signum(),
        REFEREE_AVG_HEIGHT / 2.,
    );
    let look_at = robot_pose.get_look_at_absolute(&point3);

    nao_manager.set_head_target(
        look_at,
        HEAD_ROTATION_TIME,
        Priority::default(),
        NaoManager::HEAD_STIFFNESS,
    );

    // Dont think I needs this. Only if it is necessary to update
    // the head position.
    // step_context.request_stand();

    // Request should be sended only after the HEAD_ROTATION_TIME is passed

    // Request the detection of the visual referee post
    detect_pose.send(DetectRefereePose);
}