use std::{net::SocketAddr, time::Duration};

use async_std::prelude::StreamExt;
use bevy::prelude::*;
use bifrost::{
    communication::{GameControllerReturnMessage, GAME_CONTROLLER_RETURN_PORT},
    serialization::Encode,
};
use futures::channel::mpsc::{self};
use heimdall::{Bottom, Top};

use crate::{
    core::config::showtime::PlayerConfig, localization::RobotPose, sensor::falling::FallState,
    vision::ball_detection::{ball_tracker::BallTracker, classifier::Balls, Hypothesis},
};

use super::{GameControllerConfig, GameControllerConnection, GameControllerSocket};

const NO_BALL_DETECTED_DATA: (f32, [f32; 2]) = (-1.0, [0.0, 0.0]);
const MILLIMETERS_PER_METER: f32 = 1_000.0;

#[derive(Resource)]
pub struct GameControllerSender {
    pub tx: mpsc::UnboundedSender<(GameControllerReturnMessage, SocketAddr)>,
}

pub async fn send_loop(
    sock: GameControllerSocket,
    mut rx: mpsc::UnboundedReceiver<(GameControllerReturnMessage, SocketAddr)>,
) {
    let mut buffer = Vec::new();

    while let Some((message, mut addr)) = rx.next().await {
        message.encode(&mut buffer).unwrap();
        addr.set_port(GAME_CONTROLLER_RETURN_PORT);
        sock.send_to(&buffer, addr).await.unwrap();
        buffer.clear();
    }

    tracing::error!("Exiting game controller send loop");
}

#[derive(Deref, DerefMut)]
pub struct GameControllerReturnDelay(Timer);

impl Default for GameControllerReturnDelay {
    fn default() -> Self {
        Self(Timer::new(Duration::ZERO, TimerMode::Repeating))
    }
}

pub fn send_message(
    player_config: Res<PlayerConfig>,
    fall_state: Res<FallState>,
    robot_pose: Res<RobotPose>,
    ball_tracker: Res<BallTracker>,
    (sender, connection, config): (
        Res<GameControllerSender>,
        Res<GameControllerConnection>,
        Res<GameControllerConfig>,
    ),
    time: Res<Time>,
    mut delay: Local<GameControllerReturnDelay>,
) {
    delay.tick(time.delta());

    if !delay.finished() {
        return;
    }

    let (ball_age, pall_pos) = balls_to_game_controller_ball(&ball_tracker);
    let return_message = GameControllerReturnMessage::new(
        player_config.player_number,
        player_config.team_number,
        u8::from(matches!(*fall_state, FallState::Lying(_))),
        robot_pose_to_game_controller_pose(&robot_pose),
        ball_age,
        pall_pos,
    );

    sender
        .tx
        .unbounded_send((return_message, connection.address))
        .unwrap();

    delay.set_duration(config.game_controller_return_delay);
}

fn robot_pose_to_game_controller_pose(robot_pose: &RobotPose) -> [f32; 3] {
    let robot_world_position = robot_pose.world_position();
    [
        robot_world_position.x * MILLIMETERS_PER_METER,
        robot_world_position.y * MILLIMETERS_PER_METER,
        robot_pose.world_rotation(),
    ]
}

fn balls_to_game_controller_ball(
    ball_tracker: &BallTracker
) -> (f32, [f32; 2]) {
    if let Hypothesis::Stationary = ball_tracker.cutoff() {
        return (
            ball_tracker.timestamp.elapsed().as_secs_f32(),
            [
                ball_tracker.state().0.x * MILLIMETERS_PER_METER,
                ball_tracker.state().0.y * MILLIMETERS_PER_METER,
            ],
        );
    } else {
        return NO_BALL_DETECTED_DATA;
    }
}
