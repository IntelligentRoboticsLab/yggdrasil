use bevy::{ecs::system::RunSystemOnce, prelude::*};
use nidhogg::{
    types::{FillExt, JointArray},
    DisconnectExt, HardwareInfo, NaoBackend, NaoControlMessage, NaoState, Result,
};

use nidhogg::backend::{ConnectWithRetry, ReadHardwareInfo};

use crate::nao::RobotInfo;
use crate::prelude::*;

const DEFAULT_STIFFNESS: f32 = 0.8;

/// Plugin that adds systems for reading and writing to the `LoLA` socket using [`nidhogg`].
pub(super) struct SimLolaPlugin;

impl Plugin for SimLolaPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<NaoControlMessage>();

        app.world_mut()
            .run_system_once(setup_lola)
            .expect("failed to setup lola!");
        app.world_mut()
            .run_system_once(initialize_nao)
            .expect("failed to initialize nao resources!");

        app.add_systems(Write, sync_hardware);
    }
}

/// Resource containing the [`SimLolaBackend`].
#[derive(Resource, Debug, Deref, DerefMut)]
pub struct SimLola(SimLolaBackend);

fn setup_lola(mut commands: Commands) {
    commands.insert_resource(SimLola(SimLolaBackend));
}

fn initialize_nao(mut commands: Commands, mut lola: ResMut<SimLola>) {
    // let info = RobotInfo::new(&mut lola.0).expect("failed to read robot info from LoLA");

    let info = RobotInfo {
        robot_name: "local".to_string(),
        robot_id: 27,
        head_id: Default::default(),
        head_version: Default::default(),
        body_id: Default::default(),
        body_version: Default::default(),
        initial_joint_positions: Default::default(),
    };

    // Read state and reply with a message.
    let state = lola
        .read_nao_state()
        .expect("failed to read initial state from LoLA");
    let msg = NaoControlMessage {
        position: info.initial_joint_positions.clone(),
        stiffness: JointArray::fill(DEFAULT_STIFFNESS),
        ..Default::default()
    };
    lola.send_control_msg(msg)
        .expect("failed to send initial control message to LoLA");

    tracing::info!(
        "Launched yggdrasil on {} with head_id: {}, body_id: {}",
        info.robot_name,
        info.head_id,
        info.body_id
    );

    tracing::info!("Battery level: {}", state.battery.charge);

    commands.insert_resource(state);
    commands.insert_resource(info);
}

pub fn sync_hardware(
    mut nao: ResMut<SimLola>,
    mut robot_state: ResMut<NaoState>,
    update: Res<NaoControlMessage>,
) {
    nao.send_control_msg(update.clone())
        .expect("failed to send control message to LoLA");

    *robot_state = nao
        .read_nao_state()
        .expect("failed to read state from LoLA");
}

/// `LoLA` backend that communicates with a real NAO V6 through the socket at `/tmp/robocup`
#[derive(Debug)]
pub struct SimLolaBackend;

impl NaoBackend for SimLolaBackend {
    /// Connects to a NAO backend
    ///
    /// # Examples
    /// ```no_run
    /// use nidhogg::{NaoBackend, backend::SimLolaBackend};
    ///
    /// // We connect to a real NAO using the `LoLA` backend
    /// let mut nao = SimLolaBackend::connect().expect("Could not connect to the NAO! 😪");
    /// ```
    fn connect() -> Result<Self> {
        Ok(SimLolaBackend)
    }

    /// Converts a control message to the format required by the backend and writes it to that backend.
    ///
    /// # Examples
    /// ```no_run
    /// use nidhogg::{NaoBackend, NaoControlMessage, backend::SimLolaBackend, types::color};
    ///
    /// let mut nao = SimLolaBackend::connect().unwrap();
    ///
    /// // First, create a new control message where we set the chest color
    /// let msg = NaoControlMessage::builder().chest(color::f32::MAGENTA).build();
    ///
    /// // Now we send it to the NAO!
    /// nao.send_control_msg(msg).expect("Failed to write control message to backend!");
    /// ```
    fn send_control_msg(
        &mut self,
        _: NaoControlMessage,
    ) -> std::result::Result<(), nidhogg::Error> {
        Ok(())
    }

    /// Reads the current sensor data from the chosen backend
    ///
    /// # Examples
    /// ```no_run
    /// use nidhogg::{NaoBackend, backend::SimLolaBackend};
    ///
    /// let mut nao = SimLolaBackend::connect().unwrap();
    ///
    /// // Get the current state of the robot
    /// let state = nao.read_nao_state().expect("Failed to retrieve sensor data!");
    /// ```
    fn read_nao_state(&mut self) -> Result<NaoState> {
        Ok(NaoState {
            position: Default::default(),
            stiffness: Default::default(),
            accelerometer: Default::default(),
            gyroscope: Default::default(),
            angles: Default::default(),
            sonar: Default::default(),
            fsr: Default::default(),
            touch: Default::default(),
            battery: Default::default(),
            temperature: Default::default(),
            current: Default::default(),
            status: Default::default(),
        })
    }
}

impl DisconnectExt for SimLolaBackend {
    fn disconnect(self) -> Result<()> {
        Ok(())
    }
}

impl ConnectWithRetry for SimLolaBackend {}

impl ReadHardwareInfo for SimLolaBackend {
    fn read_hardware_info(&mut self) -> Result<HardwareInfo> {
        Ok(HardwareInfo {
            body_id: Default::default(),
            body_version: Default::default(),
            head_id: Default::default(),
            head_version: Default::default(),
        })
    }
}
