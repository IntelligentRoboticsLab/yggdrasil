use std::{env, net::Ipv4Addr, str::FromStr};

use re_control_comms::viewer::ControlViewerHandle;
use sindri::config::{ConfigRobot, Robot};

pub const ROBOT_ADDRESS_ENV_KEY: &str = "RE_CONTROL_ROBOT_ADDRESS";

pub struct ConnectionState {
    pub handle: ControlViewerHandle,
    pub selected_robot_config: ConfigRobot,
    pub team_number: u8,
    pub wired_connection: bool,
    pub possible_robot_connections: Vec<ConfigRobot>,
}

impl ConnectionState {
    pub fn from_handle(handle: ControlViewerHandle) -> Self {
        let sindri_config = sindri::config::load_config().unwrap();

        let robots = sindri_config.robots;

        ConnectionState {
            handle,
            selected_robot_config: robots[0].clone(),
            team_number: sindri_config.team_number,
            wired_connection: false,
            possible_robot_connections: robots,
        }
    }

    pub fn robot_from_state(&self) -> Robot {
        self.selected_robot_config
            .clone()
            .to_robot(self.team_number, self.wired_connection)
    }
}

pub fn ip_from_env(env_key: &str) -> Ipv4Addr {
    match env::var(env_key) {
        Ok(ip_addr_str) => Ipv4Addr::from_str(&ip_addr_str)
            .unwrap_or_else(|_| panic!("{env_key} is set to an invalid ip address!")),
        Err(_) => Ipv4Addr::LOCALHOST,
    }
}
