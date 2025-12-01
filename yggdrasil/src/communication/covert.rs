use std::io::ErrorKind;
use std::net::{Ipv4Addr, SocketAddr, UdpSocket};

use bevy::prelude::*;
use rand::{seq::SliceRandom, Rng};

use crate::core::config::showtime::PlayerConfig;
use bifrost::communication::{GameControllerMessage, GamePhase, GameState, Half};

// TODO: we should expose a generic broadcast interface from team comms
const PORT_RANGE_START: u16 = 10000;

/// Per cycle interference probability
const INTERFERENCE_PROB: f32 = 0.05;

/// Plugin for leveling the playing field
pub struct InterferencePlugin;

impl Plugin for InterferencePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, interfere);
    }
}

/// System that strikes when the time is right
fn interfere(
    game_controller_message: Option<Res<GameControllerMessage>>,
    player_config: Res<PlayerConfig>,
    mut interference: Local<Option<Interference>>,
) {
    let team_number = player_config.team_number;

    if let Some(message) = game_controller_message {
        if Interference::should_engage_in_black_ops(team_number, &message) {
            let opp = message
                .teams
                .iter()
                .find(|team| team.team_number != team_number)
                .expect("are we our own biggest haters?")
                .team_number;

            let interference = interference.get_or_insert_with(|| Interference::new(opp));
            interference.interfere();
        }
    }
}

struct Interference {
    socket: UdpSocket,
    port: u16,
    received: Vec<(SocketAddr, Vec<u8>)>,
}

impl Interference {
    /// Creates a new [`Interference`].
    fn new(opp: u8) -> Self {
        let port = PORT_RANGE_START + u16::from(opp);

        let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, port)).expect("the wifi is shit");
        socket.set_nonblocking(true).expect("the kernel is shit");
        socket.set_broadcast(true).expect("literally 1984");

        Self {
            socket,
            port,
            received: Vec::new(),
        }
    }

    /// Interferes in foreign affairs.
    fn interfere(&mut self) {
        let mut buf = [0; 128];

        loop {
            match self.socket.recv_from(&mut buf) {
                Ok((len, addr)) => self.received.push((addr, buf[..len].to_vec())),
                Err(e) if e.kind() == ErrorKind::WouldBlock => break,
                Err(e) => tracing::warn!(?e, "unable to receive packet while trying to █████████"),
            }
        }

        let mut rng = rand::thread_rng();

        // never let them know your next move
        if rng.gen::<f32>() < INTERFERENCE_PROB {
            if let Some((addr, packet)) = self.received.choose(&mut rng) {
                // TODO: we could flip some bits
                self.socket.send_to(packet, (addr.ip(), self.port)).ok();
            }
        }
    }

    /// Checks & balances in accordance with the international rules-based order
    ///
    /// these were revealed to me in a dream
    fn should_engage_in_black_ops(team_number: u8, message: &GameControllerMessage) -> bool {
        // if you think about it timeouts are like ceasefires and we should respect those
        //
        // if you believe this is a bug, see <https://tinyurl.com/2ct99w9u>
        if message.game_phase != GamePhase::Normal {
            return false;
        }

        // are we seeing active combat?
        if message.state != GameState::Playing {
            return false;
        }

        // have we abandoned all hope yet?
        if message.first_half == Half::First {
            return false;
        }

        let team = message
            .team(team_number)
            .expect("in destroying the enemy, we have lost ourselves");

        // desperate times call for desperate measures
        team.score == 0
    }
}
