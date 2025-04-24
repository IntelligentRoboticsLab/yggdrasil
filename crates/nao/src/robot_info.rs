use std::env;

use crate::prelude::*;
use bevy::prelude::Resource;
use miette::IntoDiagnostic;
use nidhogg::{HardwareInfo, NaoControlMessage, backend::ReadHardwareInfo};

/// Information that uniquely identifies a robot
#[derive(Clone, Debug, Default, Resource)]
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
    pub(super) fn new<T: ReadHardwareInfo>(backend: &mut T) -> Result<Self> {
        // Read state and reply with a message.
        let state = backend.read_nao_state()?;
        let msg = NaoControlMessage {
            position: state.position.clone(),
            stiffness: state.stiffness.clone(),
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
            head_version,
            body_id,
            body_version,
        })
    }
}
