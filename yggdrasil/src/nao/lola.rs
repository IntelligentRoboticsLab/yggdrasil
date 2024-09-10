use std::time::Duration;

use bevy::prelude::*;
use nidhogg::{
    backend::LolaBackend,
    types::{FillExt, JointArray},
    NaoBackend, NaoControlMessage, NaoState,
};

use crate::nao::RobotInfo;
use crate::prelude::*;

const DEFAULT_STIFFNESS: f32 = 0.8;

#[cfg(not(feature = "local"))]
const LOLA_SOCKET_PATH: &str = "/tmp/yggdrasil";

#[cfg(feature = "local")]
const LOLA_SOCKET_PATH: &str = "/tmp/robocup";

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum LolaCycle {
    Main,
    Flush,
}

/// Plugin that adds systems for reading and writing to the LoLA socket using [`nidhogg`].
pub(super) struct LolaPlugin;

impl Plugin for LolaPlugin {
    fn build(&self, app: &mut App) {
        app.configure_sets(Update, LolaCycle::Main)
            .configure_sets(Write, LolaCycle::Flush);

        app.add_systems(Startup, (setup_lola, initialize_nao).chain());
        app.add_systems(Write, write_hardware_info.in_set(LolaCycle::Flush));
    }
}

/// Resource containing the [`LolaBackend`].
#[derive(Resource, Debug)]
struct Lola(LolaBackend);

fn setup_lola(mut commands: Commands) {
    let nao =
        LolaBackend::connect_with_path_with_retry(10, Duration::from_millis(500), LOLA_SOCKET_PATH)
            .expect("failed to open connection to LoLA");

    commands.insert_resource(Lola(nao));
}

fn initialize_nao(mut commands: Commands, mut lola: ResMut<Lola>) -> Result<()> {
    let info = RobotInfo::new(&mut lola.0)?;

    // Read state and reply with a message.
    let state = lola.read_nao_state()?;
    let msg = NaoControlMessage {
        position: info.initial_joint_positions.clone(),
        stiffness: JointArray::fill(DEFAULT_STIFFNESS),
        ..Default::default()
    };
    lola.send_control_msg(msg)?;

    tracing::info!(
        "Launched yggdrasil on {} with head_id: {}, body_id: {}",
        info.robot_name,
        info.head_id,
        info.body_id
    );

    tracing::info!("Battery level: {}", state.battery.charge);

    commands.insert_resource(Resource::new(state))?;
    commands.insert_resource(Resource::new(info))?;

    Ok(())
}

pub fn write_hardware_info(
    mut nao: ResMut<Lola>,
    mut robot_state: ResMut<NaoState>,
    update: Res<NaoControlMessage>,
) -> Result<()> {
    *robot_state = nao.read_nao_state()?;

    nao.send_control_msg(update.clone())?;

    Ok(())
}
