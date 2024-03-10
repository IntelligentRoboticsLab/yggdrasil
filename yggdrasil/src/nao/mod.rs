mod cycle;
pub use cycle::*;

use crate::prelude::*;
use std::{env, time::Duration};

use miette::IntoDiagnostic;
use nidhogg::{
    backend::{ConnectWithRetry, LolaBackend, ReadHardwareInfo},
    HardwareInfo, NaoBackend, NaoControlMessage, NaoState,
};

const SOCKET_PATH: &str = "/tmp/yggdrasil";

#[derive(Clone, Debug)]
/// Information that uniquely identifies a robot
pub struct RobotInfo {
    /// Name of the robot
    pub robot_name: String,
    /// Robot id/number used to assign IP
    pub robot_id: u32,
    /// Unique hardware id of the head
    pub head_id: String,
    /// Hardware version of the head
    pub head_version: String,
    /// Unique hardware id of the body
    pub body_id: String,
    /// Hardware version of the body
    pub body_version: String,
}

impl RobotInfo {
    fn new<T: ReadHardwareInfo>(backend: &mut T) -> Result<Self> {
        let HardwareInfo {
            body_id,
            head_id,
            body_version,
            head_version,
        } = backend.read_hardware_info()?;

        let robot_name = env::var("ROBOT_NAME").into_diagnostic()?;
        let robot_id = str::parse(&env::var("ROBOT_ID").into_diagnostic()?).into_diagnostic()?;

        Ok(Self {
            robot_name,
            robot_id,
            head_id,
            body_id,
            head_version,
            body_version,
        })
    }
}

/// This module provides the following resources to the application:
/// - [`LolaBackend`]
/// - [`NaoState`]
/// - [`NaoControlMessage`]
/// - [`RobotInfo`]
/// - [`CycleTime`]
pub struct NaoModule;

impl Module for NaoModule {
    fn initialize(self, app: App) -> Result<App> {
        Ok(app
            .add_startup_system(initialize_nao)?
            .init_resource::<NaoControlMessage>()?
            .add_system(write_hardware_info)
            .add_startup_system(cycle::initialize_cycle_counter)?
            .add_system(cycle::update_cycle_stats.after(write_hardware_info)))
    }
}

#[startup_system]
fn initialize_nao(storage: &mut Storage) -> Result<()> {
    let mut nao =
        LolaBackend::connect_with_retry(10, Duration::from_millis(500), Some(SOCKET_PATH))?;

    let info = RobotInfo::new(&mut nao)?;
    let state = nao.read_nao_state()?;

    tracing::info!(
        "Launched yggdrasil on {} with head_id: {}, body_id: {}",
        info.robot_name,
        info.head_id,
        info.body_id
    );

    tracing::info!("Battery level: {}", state.battery.charge,);

    storage.add_resource(Resource::new(nao))?;
    storage.add_resource(Resource::new(state))?;
    storage.add_resource(Resource::new(info))?;

    Ok(())
}

#[system]
pub fn write_hardware_info(
    nao: &mut LolaBackend,
    robot_state: &mut NaoState,
    update: &NaoControlMessage,
) -> Result<()> {
    *robot_state = nao.read_nao_state()?;
    println!("Finished reading");
    nao.send_control_msg(update.clone())?;
    println!("Finished writing");
    Ok(())
}
