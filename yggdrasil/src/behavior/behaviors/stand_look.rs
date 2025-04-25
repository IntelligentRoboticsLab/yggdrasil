use bevy::prelude::*;
use nalgebra::{Point2, Point3};
use std::time::Duration;

use crate::{
    behavior::engine::{Behavior, BehaviorState, in_behavior},
    localization::RobotPose,
    motion::walking_engine::{StandingHeight, step_context::StepContext},
    nao::{NaoManager, Priority},
};

const HEAD_ROTATION_TIME: Duration = Duration::from_millis(500);

/// Stand and look at a target point.
/// This is used for when the robot is in the Set state.
#[derive(Resource)]
pub struct StandLookAt {
    pub target: Point2<f32>,
}

impl Behavior for StandLookAt {
    const STATE: BehaviorState = BehaviorState::StandLookAt;
}

pub struct StandLookAtBehaviorPlugin;
impl Plugin for StandLookAtBehaviorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, stand_look_at.run_if(in_behavior::<StandLookAt>));
    }
}

fn stand_look_at(
    stand_look_at: Res<StandLookAt>,
    pose: Res<RobotPose>,
    mut nao_manager: ResMut<NaoManager>,
    mut step_context: ResMut<StepContext>,
) {
    let point3 = Point3::new(
        stand_look_at.target.x,
        stand_look_at.target.y,
        RobotPose::CAMERA_HEIGHT,
    );
    let look_at = pose.get_look_at_absolute(&point3);

    nao_manager.set_head_target(
        look_at,
        HEAD_ROTATION_TIME,
        Priority::default(),
        NaoManager::HEAD_STIFFNESS,
    );
    step_context.request_stand_with_height(StandingHeight::MAX);
}
