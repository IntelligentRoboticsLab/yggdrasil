use bevy::prelude::*;
use nalgebra::Point2;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use tracing::warn;

use crate::{
    behavior::{
        BehaviorConfig,
        engine::{Behavior, BehaviorState, in_behavior},
    },
    motion::walking_engine::{step::Step, step_context::StepContext},
    nao::{NaoManager, Priority},
    vision::ball_detection::ball_tracker::BallTracker,
};
use nidhogg::types::{FillExt, HeadJoints};

const ROTATION_STIFFNESS: f32 = 0.3;

/// Config struct containing parameters for the initial behavior.
#[serde_as]
#[derive(Resource, Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct LostBallSearchBehaviorConfig {
    // Controls how fast the robot moves its head back and forth while looking around
    pub head_rotation_speed: f32,
    // Controls how far to the left and right the robot looks while looking around, in radians.
    // If this value is one, the robot will look one radian to the left and one radian to the
    // right.
    pub head_pitch_max: f32,
    // Controls how far to the bottom the robot looks while looking around, in radians
    pub head_yaw_max: f32,
}

const LOST_BALL_SEARCH_TIME: f32 = 10.0;

#[derive(Resource)]
pub struct LostBallSearchTimer {
    pub timer: Timer,
    pub last_ball: Point2<f32>,
}

impl LostBallSearchTimer {
    pub fn new(last_ball: Point2<f32>) -> Self {
        LostBallSearchTimer {
            timer: Timer::from_seconds(LOST_BALL_SEARCH_TIME, TimerMode::Once),
            last_ball,
        }
    }
}
/// This behavior makes the robot look around with a sinusoidal head movement with an optional step.
/// With this behavior, the robot can observe its surroundings while standing still or turning.
#[derive(Resource, Default)]
pub struct LostBallSearch;

// impl LostBallSearch {
//     #[must_use]
//     pub fn with_turning(turn: f32) -> Self {
//         LostBallSearch {
//             step: Some(Step {
//                 turn,
//                 ..Default::default()
//             }),
//         }
//     }
// }

impl Behavior for LostBallSearch {
    const STATE: BehaviorState = BehaviorState::LostBallSearch;
}

pub struct LostBallSearchBehaviorPlugin;

impl Plugin for LostBallSearchBehaviorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, search.run_if(in_behavior::<LostBallSearch>))
            .add_systems(
                OnEnter(BehaviorState::LostBallSearch),
                init_ball_search_starting_time,
            )
            .add_systems(
                OnExit(BehaviorState::LostBallSearch),
                |mut commands: Commands| {
                    commands.remove_resource::<LostBallSearchTimer>();
                },
            );
    }
}

fn init_ball_search_starting_time(mut commands: Commands, ball_tracker: Res<BallTracker>) {
    let Some(relative_ball) = ball_tracker.stationary_ball() else {
        warn!("We should not be able to reach this part");
        return;
    };

    println!("LostBallSearch INIT last ball: {relative_ball:?}");

    commands.insert_resource(LostBallSearchTimer {
        last_ball: relative_ball,
        timer: Timer::from_seconds(LOST_BALL_SEARCH_TIME, TimerMode::Once),
    });
}

fn search(
    mut nao_manager: ResMut<NaoManager>,
    behavior_config: Res<BehaviorConfig>,
    // observe: Res<LostBallSearch>,
    mut observe_starting_time: ResMut<LostBallSearchTimer>,
    mut step_context: ResMut<StepContext>,
    time: Res<Time>,
) {
    println!(
        "EXECUTING LOST BALL SEARCH: {:?}, time since last ball: {:}",
        observe_starting_time.last_ball.coords,
        observe_starting_time.timer.elapsed().as_secs_f32()
    );
    observe_starting_time.timer.tick(time.delta()); // <- tick the timer

    let observe_config = &behavior_config.observe;
    look_around(
        &mut nao_manager,
        &observe_starting_time.timer,
        observe_config.head_rotation_speed,
        observe_config.head_yaw_max,
        observe_config.head_pitch_max,
    );

    // if let Some(step) = observe.step {
    //     step_context.request_walk(step);
    // } else {
    //     step_context.request_stand_with_height(StandingHeight::MAX);
    // }

    step_context.request_walk(Step {
        turn: observe_starting_time.last_ball.y.signum() * 0.4,
        ..Default::default()
    });
}

fn look_around(
    nao_manager: &mut NaoManager,
    timer: &Timer,
    rotation_speed: f32,
    yaw_multiplier: f32,
    pitch_multiplier: f32,
) {
    // Used to parameterize the yaw and pitch angles, multiplying with a large
    // rotation speed will make the rotation go faster.
    let movement_progress = timer.elapsed().as_secs_f32() * rotation_speed;
    let yaw = (movement_progress).sin() * yaw_multiplier;
    let pitch = (movement_progress * 2.0 + std::f32::consts::FRAC_PI_2)
        .sin()
        .max(0.0)
        * pitch_multiplier;

    let position = HeadJoints { yaw, pitch };
    let stiffness = HeadJoints::fill(ROTATION_STIFFNESS);

    nao_manager.set_head(position, stiffness, Priority::default());
}
