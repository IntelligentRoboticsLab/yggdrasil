use super::GameControllerConfig;
use super::GameControllerData;

use bifrost::communication::{GameControllerReturnMessage, GAME_CONTROLLER_RETURN_PORT};
use bifrost::serialization::Encode;

use tokio::net::UdpSocket;
use tokio::time::sleep;

use std::mem::size_of;
use std::net::SocketAddr;
use std::ops::Add;
use std::sync::Arc;
use std::time::{Duration, Instant};

use miette::IntoDiagnostic;

use crate::core::config::showtime::PlayerConfig;
use crate::localization::RobotPose;
use crate::prelude::*;
use crate::sensor::falling::FallState;
use crate::vision::ball_detection::classifier::Balls;

const NO_BALL_DETECTED_DATA: (f32, [f32; 2]) = (-1.0, [0.0, 0.0]);
const MILIMETERS_PER_METER: f32 = 1_000.;

pub(super) struct GameControllerTransmitModule;

impl Module for GameControllerTransmitModule {
    fn initialize(self, app: App) -> Result<App> {
        Ok(app
            .add_task::<AsyncTask<Result<(GameControllerReturnMessage, Instant)>>>()?
            .add_system(transmit_system))
    }
}

struct TransmitGameControllerData {
    player_num: u8,
    team_num: u8,
    fallen: u8,
    pose: [f32; 3],
    ball_age: f32,
    ball: [f32; 2],
}

async fn transmit_game_controller_return_message(
    game_controller_socket: Arc<UdpSocket>,
    last_transmitted_return_message: Instant,
    mut game_controller_address: SocketAddr,
    game_controller_return_delay: Duration,
    transmit_game_controller_data: TransmitGameControllerData,
) -> Result<(GameControllerReturnMessage, Instant)> {
    let duration_to_wait = last_transmitted_return_message
        .add(game_controller_return_delay)
        .duration_since(Instant::now());
    sleep(duration_to_wait).await;

    let mut message_buffer = [0u8; size_of::<GameControllerReturnMessage>()];
    let game_controller_message = GameControllerReturnMessage::new(
        transmit_game_controller_data.player_num,
        transmit_game_controller_data.team_num,
        transmit_game_controller_data.fallen,
        transmit_game_controller_data.pose,
        transmit_game_controller_data.ball_age,
        transmit_game_controller_data.ball,
    );
    game_controller_message
        .encode(message_buffer.as_mut_slice())
        .into_diagnostic()?;

    game_controller_address.set_port(GAME_CONTROLLER_RETURN_PORT);
    game_controller_socket
        .send_to(message_buffer.as_slice(), game_controller_address)
        .await
        .into_diagnostic()?;

    Ok((game_controller_message, Instant::now()))
}

fn robot_pose_to_game_controller_pose(robot_pose: &RobotPose) -> [f32; 3] {
    let robot_world_position = robot_pose.world_position();
    [
        robot_world_position.x * MILIMETERS_PER_METER,
        robot_world_position.y * MILIMETERS_PER_METER,
        robot_pose.world_rotation(),
    ]
}

fn balls_to_game_controller_ball(balls: &Balls) -> (f32, [f32; 2]) {
    let Some(ball) = balls.most_confident_ball() else {
        return NO_BALL_DETECTED_DATA;
    };

    (
        ball.timestamp.elapsed().as_secs_f32(),
        [
            ball.robot_to_ball.x * MILIMETERS_PER_METER,
            ball.robot_to_ball.y * MILIMETERS_PER_METER,
        ],
    )
}

#[system]
fn transmit_system(
    game_controller_data: &mut GameControllerData,
    transmit_game_controller_return_message_task: &mut AsyncTask<
        Result<(GameControllerReturnMessage, Instant)>,
    >,
    fall_state: &FallState,
    game_controller_config: &GameControllerConfig,
    player_config: &PlayerConfig,
    robot_pose: &RobotPose,
    balls: &Balls,
) -> Result<()> {
    let Some((game_controller_address, mut last_transmitted_update_timestamp)) =
        game_controller_data.game_controller_address
    else {
        return Ok(());
    };

    let game_controller_ball_data = balls_to_game_controller_ball(balls);

    let transmit_game_controller_data = TransmitGameControllerData {
        player_num: player_config.player_number,
        team_num: player_config.team_number,
        fallen: matches!(fall_state, FallState::Lying(_)) as u8,
        pose: robot_pose_to_game_controller_pose(robot_pose),
        ball_age: game_controller_ball_data.0,
        ball: game_controller_ball_data.1,
    };

    match transmit_game_controller_return_message_task.poll() {
        Some(Ok((_game_controller_return_message, new_last_transmitted_timestamp))) => {
            last_transmitted_update_timestamp = new_last_transmitted_timestamp;
        }
        Some(Err(error)) => {
            tracing::warn!("Failed to transmit game controller return message: {error}");
        }
        None => (),
    }

    _ = transmit_game_controller_return_message_task.try_spawn(
        transmit_game_controller_return_message(
            game_controller_data.socket.clone(),
            last_transmitted_update_timestamp,
            game_controller_address,
            game_controller_config.game_controller_return_delay,
            transmit_game_controller_data,
        ),
    );

    Ok(())
}
