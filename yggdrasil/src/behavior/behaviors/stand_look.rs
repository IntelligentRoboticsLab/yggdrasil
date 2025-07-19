use bevy::prelude::*;
use nalgebra::{Point2, Point3};

use crate::{
    behavior::engine::{Behavior, BehaviorState, in_behavior},
    localization::RobotPose,
    motion::walking_engine::{StandingHeight, step_context::StepContext},
    nao::{HeadMotionManager, LookAt},
};

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
    mut step_context: ResMut<StepContext>,
    mut head_motion_manager: ResMut<HeadMotionManager>,
) {
    let point3 = Point3::new(
        stand_look_at.target.x,
        stand_look_at.target.y,
        RobotPose::CAMERA_HEIGHT,
    );

    head_motion_manager.request_look_at(LookAt {
        pose: *pose,
        point: point3,
    });

    step_context.request_stand_with_height(StandingHeight::MAX);
}
