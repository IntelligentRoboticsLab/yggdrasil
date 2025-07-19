use std::time::Duration;

use bevy::prelude::*;
use nalgebra::Point3;

use crate::{
    behavior::engine::{Behavior, BehaviorState, in_behavior},
    core::config::layout::LayoutConfig,
    localization::RobotPose,
    nao::{HeadMotionManager, LookAt},
    vision::referee::recognize::{RecognizeRefereePose, VisualRefereeRecognitionStatus},
};

const REFEREE_AVG_HEIGHT: f32 = 1.62;
const HEAD_ROTATION_TIME: Duration = Duration::from_millis(500);

pub struct VisualRefereeBehaviorPlugin;

impl Plugin for VisualRefereeBehaviorPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(VisualRefHeadRotationTimer::new(HEAD_ROTATION_TIME))
            .add_systems(
                Update,
                (
                    detect_visual_referee.run_if(
                        in_behavior::<VisualReferee>
                            .and(in_state(VisualRefereeRecognitionStatus::Inactive)),
                    ),
                    reset_head_rotation_timer.run_if(not(in_behavior::<VisualReferee>)),
                ),
            );
    }
}

#[derive(Resource)]
pub struct VisualReferee;

impl Behavior for VisualReferee {
    const STATE: BehaviorState = BehaviorState::VisualReferee;
}

#[derive(Resource)]
struct VisualRefHeadRotationTimer {
    timer: Timer,
}

impl VisualRefHeadRotationTimer {
    fn new(duration: Duration) -> Self {
        VisualRefHeadRotationTimer {
            timer: Timer::new(duration, TimerMode::Once),
        }
    }
}

fn detect_visual_referee(
    layout_config: Res<LayoutConfig>,
    robot_pose: Res<RobotPose>,
    mut recognize_pose: EventWriter<RecognizeRefereePose>,
    mut head_motion_manager: ResMut<HeadMotionManager>,
    mut timer: ResMut<VisualRefHeadRotationTimer>,
    time: Res<Time>,
) {
    // Make the robot look at the opposite T junction (relative to the starting)
    // position.
    let field_y_max = layout_config.field.width / 2.;
    let world_position = robot_pose.world_position();

    let point3 = Point3::new(
        0.,
        -field_y_max * world_position.y.signum(),
        REFEREE_AVG_HEIGHT / 2.,
    );

    head_motion_manager.request_look_at(LookAt {
        pose: *robot_pose,
        point: point3,
    });

    timer.timer.tick(time.delta());

    // Request should be sent only after the HEAD_ROTATION_TIME is passed
    if timer.timer.finished() {
        // Request the detection of the visual referee post
        recognize_pose.write(RecognizeRefereePose);
    }
}

// Reset the head rotation timer. Runs when not in the behavior `VisualReferee`
// and resets when the timer already started.
fn reset_head_rotation_timer(mut timer: ResMut<VisualRefHeadRotationTimer>) {
    if timer.timer.elapsed_secs() > 0.0 {
        timer.timer.reset();
    }
}
