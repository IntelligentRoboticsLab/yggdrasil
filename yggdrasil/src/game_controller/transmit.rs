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

use crate::prelude::*;

const GAME_CONTROLLER_RETURN_DELAY: Duration = Duration::from_millis(500);

pub(super) struct GameControllerTransmitModule;

impl Module for GameControllerTransmitModule {
    fn initialize(self, app: App) -> Result<App> {
        Ok(app
            .add_task::<AsyncTask<Result<(GameControllerReturnMessage, Instant)>>>()?
            .add_system(transmit_system))
    }
}

async fn transmit_game_controller_return_message(
    game_controller_socket: Arc<UdpSocket>,
    last_transmitted_return_message: Instant,
    mut game_controller_address: SocketAddr,
) -> Result<(GameControllerReturnMessage, Instant)> {
    let duration_to_wait = last_transmitted_return_message
        .add(GAME_CONTROLLER_RETURN_DELAY)
        .duration_since(Instant::now());
    sleep(duration_to_wait).await;

    // TODO: Substitute with real data from resources and/or configs.
    let robot_number = 2;
    let team_number = 8;
    let fallen = false;
    let pose = [0f32; 3];
    let ball_age = -1f32;
    let ball_position = [0f32; 2];

    let mut message_buffer = [0u8; size_of::<GameControllerReturnMessage>()];
    let game_controller_message = GameControllerReturnMessage::new(
        robot_number,
        team_number,
        fallen as u8,
        pose,
        ball_age,
        ball_position,
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

#[system]
fn transmit_system(
    game_controller_data: &mut GameControllerData,
    transmit_game_controller_return_message_task: &mut AsyncTask<
        Result<(GameControllerReturnMessage, Instant)>,
    >,
) -> Result<()> {
    let Some((game_controller_address, mut last_transmitted_update_timestamp)) =
        game_controller_data.game_controller_address
    else {
        return Ok(());
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
        ),
    );

    Ok(())
}
