use std::{net::SocketAddr, time::Duration};

use async_std::prelude::StreamExt;
use bevy::prelude::*;
use bifrost::{
    communication::{GameControllerReturnMessage, GAME_CONTROLLER_RETURN_PORT},
    serialization::Encode,
};
use futures::channel::mpsc::{self};

use crate::{
    core::config::showtime::PlayerConfig, localization::RobotPose, sensor::falling::FallState,
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
    // balls: Res<Balls>,
    sender: Res<GameControllerSender>,
    connection: Res<GameControllerConnection>,
    cfg: Res<GameControllerConfig>,
    time: Res<Time>,
    mut delay: Local<GameControllerReturnDelay>,
) {
    delay.tick(time.delta());

    if delay.finished() {
        // TODO: return the balls
        // let game_controller_ball_data = balls_to_game_controller_ball(balls);

        let return_message = GameControllerReturnMessage::new(
            player_config.player_number,
            player_config.team_number,
            matches!(*fall_state, FallState::Lying(_)) as u8,
            robot_pose_to_game_controller_pose(&robot_pose),
            NO_BALL_DETECTED_DATA.0,
            NO_BALL_DETECTED_DATA.1,
        );

        sender
            .tx
            .unbounded_send((return_message, connection.address))
            .unwrap();

        delay.set_duration(cfg.game_controller_return_delay);
    }
}

fn robot_pose_to_game_controller_pose(robot_pose: &RobotPose) -> [f32; 3] {
    let robot_world_position = robot_pose.world_position();
    [
        robot_world_position.x * MILLIMETERS_PER_METER,
        robot_world_position.y * MILLIMETERS_PER_METER,
        robot_pose.world_rotation(),
    ]
}

// fn balls_to_game_controller_ball(balls: &Balls) -> (f32, [f32; 2]) {
//     let Some(ball) = balls.most_confident_ball() else {
//         return NO_BALL_DETECTED_DATA;
//     };

//     (
//         ball.timestamp.elapsed().as_secs_f32(),
//         [
//             ball.robot_to_ball.x * MILLIMETERS_PER_METER,
//             ball.robot_to_ball.y * MILLIMETERS_PER_METER,
//         ],
//     )
// }
