use std::collections::HashMap;
use std::io;
use std::net::{Ipv4Addr, SocketAddr, UdpSocket};
use std::time::{Duration, Instant};

use miette::IntoDiagnostic;

use crate::config::showtime::ShowtimeConfig;
use crate::prelude::*;
use bifrost::communication::GameControllerMessage;
use bifrost::serialization::{Decode, Encode};

const PORT_RANGE_START: u16 = 10000;
const INTERVAL: Duration = Duration::from_secs(1);
const SAVINGS: u16 = 8;

pub struct RobotToRobotModule;

impl Module for RobotToRobotModule {
    fn initialize(self, app: App) -> Result<App> {
        Ok(app
            .add_startup_system(init_robot_to_robot)?
            .add_system(sync_shared_state))
    }
}

#[startup_system]
fn init_robot_to_robot(storage: &mut Storage, config: &ShowtimeConfig) -> Result<()> {
    let rtr = RobotToRobot::new(config.team_number);
    storage.add_resource(Resource::new(rtr?))
}

#[system]
fn sync_shared_state(rtr: &mut RobotToRobot, message: &Option<GameControllerMessage>) -> Result<()> {
    let mut buf = [0; 128];

    // Retrieve the remaining packet budget.
    let budget = match message {
        Some(message) => match message.teams.iter().find(|t| t.team_number == rtr.team_number) {
            Some(team) => team.message_budget,
            None => 0, // TODO: This shouldn't happen, so we should throw an error.
        },
        None => 0,
    };

    // Time to send the next update?
    if rtr.should_sync(budget) {
        rtr.state.encode(&mut buf[..]).into_diagnostic()?;

        match rtr.socket.send_to(&buf, (Ipv4Addr::BROADCAST, rtr.port)) {
            Ok(_) => {
                rtr.last = Instant::now();
                rtr.out_of_sync = false;
            }
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => (),
            Err(e) => return Err(e).into_diagnostic(),
        }
    }

    // Check if we have received updates from our peers.
    match rtr.socket.recv_from(&mut buf) {
        Ok((len, addr)) => {
            let state = SharedState::decode(&buf[..len]).into_diagnostic()?;
            tracing::info!("received {:?} from {:?}", state, addr);

            rtr.peers.insert(addr, state);
        }
        Err(e) if e.kind() == io::ErrorKind::WouldBlock => (),
        Err(e) => return Err(e).into_diagnostic(),
    }

    Ok(())
}

pub struct RobotToRobot {
    port: u16,
    team_number: u8,
    socket: UdpSocket,
    last: Instant,
    state: SharedState,
    peers: HashMap<SocketAddr, SharedState>,
    out_of_sync: bool,
}

impl RobotToRobot {
    fn new(team_number: u8) -> Result<Self> {
        let port = PORT_RANGE_START + team_number as u16;

        let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, port)).into_diagnostic()?;
        socket.set_nonblocking(true).into_diagnostic()?;
        socket.set_broadcast(true).into_diagnostic()?;

        Ok(Self {
            port,
            team_number,
            socket,
            last: Instant::now(),
            state: Default::default(),
            peers: HashMap::new(),
            out_of_sync: false,
        })
    }

    pub fn should_sync(&self, budget: u16) -> bool {
        self.last.elapsed() >= INTERVAL && budget > SAVINGS && self.out_of_sync
    }

    pub fn state(&self) -> &SharedState {
        &self.state
    }

    pub fn state_mut(&mut self) -> &mut SharedState {
        self.out_of_sync = true;
        &mut self.state
    }

    pub fn peers(&self) -> &HashMap<SocketAddr, SharedState> {
        &self.peers
    }
}

#[derive(Debug, Default, Decode, Encode)]
pub struct SharedState {
    pub last_whistle: u64,
}
