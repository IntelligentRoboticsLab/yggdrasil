use super::GameControllerData;

use bifrost::communication::{RoboCupGameControlReturnData, GAMECONTROLLER_RETURN_PORT};
use bifrost::serialization::Encode;
use tokio::net::UdpSocket;
use tokio::time::sleep;

use std::net::SocketAddr;
use std::ops::Add;
use std::sync::Arc;
use std::time::{Duration, Instant};

use miette::{IntoDiagnostic, Result};
use tyr::prelude::*;

const GAMECONTROLLER_RETURN_DELAY_MS: u64 = 500;

pub struct GameControllerSendModule;

impl Module for GameControllerSendModule {
    fn initialize(self, app: App) -> Result<App> {
        Ok(app
            .add_task::<AsyncTask<Result<(RoboCupGameControlReturnData, Instant)>>>()?
            .add_system(send_system))
    }
}

async fn transmit_game_controller_return_data(
    game_controller_socket: Arc<UdpSocket>,
    last_send_return_message: Instant,
    mut game_controller_address: SocketAddr,
) -> Result<(RoboCupGameControlReturnData, Instant)> {
    let mut buffer = [0u8; 1024];

    let duration_to_wait = last_send_return_message
        .add(Duration::from_millis(GAMECONTROLLER_RETURN_DELAY_MS))
        .duration_since(Instant::now());

    sleep(duration_to_wait).await;

    // TODO: Substitute with real data from resources and/or configs.
    let robot_number = 2;
    let team_number = 8;
    let fallen = false;
    let pose = [0f32; 3];
    let ball_age = -1f32;
    let ball_position = [0f32; 2];

    let game_controller_message = RoboCupGameControlReturnData::new(
        robot_number,
        team_number,
        fallen as u8,
        pose,
        ball_age,
        ball_position,
    );
    game_controller_message
        .encode(buffer.as_mut_slice())
        .into_diagnostic()?;

    game_controller_address.set_port(GAMECONTROLLER_RETURN_PORT);
    game_controller_socket
        .send_to(buffer.as_slice(), game_controller_address)
        .await
        .into_diagnostic()?;

    Ok((game_controller_message, Instant::now()))
}

#[system]
pub(crate) fn send_system(
    game_controller_data: &mut GameControllerData,
    transmit_game_controller_return_data_task: &mut AsyncTask<
        Result<(RoboCupGameControlReturnData, Instant)>,
    >,
) -> Result<()> {
    let Some(game_controller_address) = game_controller_data.game_controller_address else {
        return Ok(());
    };

    if !transmit_game_controller_return_data_task.active() {
        transmit_game_controller_return_data_task
            .try_spawn(transmit_game_controller_return_data(
                game_controller_data.socket.clone(),
                game_controller_data.last_send_message_instant,
                game_controller_address,
            ))
            .into_diagnostic()?;
    } else {
        match transmit_game_controller_return_data_task.poll() {
            Some(Ok((_game_controller_return_message, last_transmitted_instant))) => {
                game_controller_data.last_send_message_instant = last_transmitted_instant
            }
            Some(Err(error)) => {
                tracing::warn!("Failed to transmit game controller return data: {error}");
            }
            None => {}
        }
    }

    Ok(())
}
