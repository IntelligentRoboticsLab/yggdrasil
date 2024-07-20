mod cycle;
pub use cycle::*;

mod battery_led;

use battery_led::battery_display;
pub mod center_of_mass;
pub mod manager;

use crate::prelude::*;
use std::{env, time::Duration};

use miette::IntoDiagnostic;
use nidhogg::{
    backend::{LolaBackend, ReadHardwareInfo},
    types::{FillExt, JointArray},
    HardwareInfo, NaoBackend, NaoControlMessage, NaoState,
};

const DEFAULT_STIFFNESS: f32 = 0.8;

#[cfg(not(feature = "local"))]
const LOLA_SOCKET_PATH: &str = "/tmp/yggdrasil";

#[cfg(feature = "local")]
const LOLA_SOCKET_PATH: &str = "/tmp/robocup";

#[derive(Clone, Debug, Default)]
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
    /// Initial joint positions
    pub initial_joint_positions: JointArray<f32>,
}

impl RobotInfo {
    fn new<T: ReadHardwareInfo>(backend: &mut T) -> Result<Self> {
        // Read state and reply with a message.
        let state = backend.read_nao_state()?;
        let msg = NaoControlMessage {
            position: state.position.clone(),
            stiffness: JointArray::fill(DEFAULT_STIFFNESS),
            ..Default::default()
        };
        backend.send_control_msg(msg.clone())?;

        // Read hardware info and reply with a message.
        let HardwareInfo {
            body_id,
            head_id,
            body_version,
            head_version,
        } = backend.read_hardware_info()?;
        backend.send_control_msg(msg)?;

        let robot_name = env::var("ROBOT_NAME").into_diagnostic()?;
        let robot_id = str::parse(&env::var("ROBOT_ID").into_diagnostic()?).into_diagnostic()?;

        Ok(Self {
            robot_name,
            robot_id,
            head_id,
            body_id,
            head_version,
            body_version,
            initial_joint_positions: state.position,
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
            .add_module(manager::NaoManagerModule)?
            .add_module(center_of_mass::CenterOfMassModule)?
            .add_staged_system(SystemStage::Write, write_hardware_info)
            .add_startup_system(cycle::initialize_cycle_counter)?
            .add_staged_system(
                SystemStage::Write,
                cycle::update_cycle_stats.after(write_hardware_info),
            )
            .add_system(battery_display))
    }
}

#[startup_system]
fn initialize_nao(storage: &mut Storage) -> Result<()> {
    let mut nao = LolaBackend::connect_with_path_with_retry(
        10,
        Duration::from_millis(500),
        LOLA_SOCKET_PATH,
    )?;
    let info = RobotInfo::new(&mut nao)?;

    // Read state and reply with a message.
    let state = nao.read_nao_state()?;
    let msg = NaoControlMessage {
        position: info.initial_joint_positions.clone(),
        stiffness: JointArray::fill(DEFAULT_STIFFNESS),
        ..Default::default()
    };
    nao.send_control_msg(msg)?;

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

    nao.send_control_msg(update.clone())?;

    Ok(())
}
