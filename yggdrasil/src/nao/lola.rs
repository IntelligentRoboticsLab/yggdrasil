use std::time::Duration;

use bevy::{ecs::system::RunSystemOnce, prelude::*};
use nidhogg::{NaoBackend, NaoControlMessage, NaoState, backend::LolaBackend};

use crate::core::debug::serialized_component_batch_f32;
use crate::{core::debug, prelude::*};
use crate::{core::debug::DebugContext, nao::RobotInfo};

use super::Cycle;

#[cfg(not(feature = "local"))]
const LOLA_SOCKET_PATH: &str = "/tmp/yggdrasil";

#[cfg(feature = "local")]
const LOLA_SOCKET_PATH: &str = "/tmp/robocup";

/// Plugin that adds systems for reading and writing to the `LoLA` socket using [`nidhogg`].
pub(super) struct LolaPlugin;

impl Plugin for LolaPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<NaoControlMessage>();

        app.world_mut()
            .run_system_once(setup_lola)
            .expect("failed to setup lola!");
        app.world_mut()
            .run_system_once(initialize_nao)
            .expect("failed to initialize nao resources!");

        app.add_systems(
            Write,
            (
                log_nao_state.run_if(debug::logging_to_file_sink),
                sync_hardware,
                log_nao_control_message.run_if(debug::logging_to_file_sink),
            )
                .chain(),
        );
    }
}

/// Resource containing the [`LolaBackend`].
#[derive(Resource, Debug, Deref, DerefMut)]
pub struct Lola(LolaBackend);

fn setup_lola(mut commands: Commands) {
    let nao =
        LolaBackend::connect_with_path_with_retry(10, Duration::from_millis(500), LOLA_SOCKET_PATH)
            .expect("failed to open connection to LoLA");

    commands.insert_resource(Lola(nao));
}

fn initialize_nao(mut commands: Commands, mut lola: ResMut<Lola>) {
    let info = RobotInfo::new(&mut lola.0).expect("failed to read robot info from LoLA");

    // Read state and reply with a message.
    let state = lola
        .read_nao_state()
        .expect("failed to read initial state from LoLA");
    let msg = NaoControlMessage {
        position: state.position.clone(),
        stiffness: state.stiffness.clone(),
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
    mut nao: ResMut<Lola>,
    mut robot_state: ResMut<NaoState>,
    update: Res<NaoControlMessage>,
) {
    nao.send_control_msg(update.clone())
        .expect("failed to send control message to LoLA");

    *robot_state = nao
        .read_nao_state()
        .expect("failed to read state from LoLA");
}

fn log_nao_state(ctx: DebugContext, cycle: Res<Cycle>, nao_state: Res<NaoState>) {
    let joint_positions = serialized_component_batch_f32(
        "yggdrasil.components.JointPosition",
        nao_state.position.clone(),
    );
    let joint_stiffness = serialized_component_batch_f32(
        "yggdrasil.components.JointStiffness",
        nao_state.stiffness.clone(),
    );

    let currents = serialized_component_batch_f32(
        "yggdrasil.components.JointCurrent",
        nao_state.current.clone(),
    );

    let temperature = serialized_component_batch_f32(
        "yggdrasil.components.JointTemperature",
        nao_state.temperature.clone(),
    );

    ctx.log_with_cycle(
        "nao/state",
        *cycle,
        &[joint_positions, joint_stiffness, currents, temperature],
    );
}

fn log_nao_control_message(
    ctx: DebugContext,
    cycle: Res<Cycle>,
    control_msg: Res<NaoControlMessage>,
) {
    let joint_positions = serialized_component_batch_f32(
        "yggdrasil.components.JointPosition",
        control_msg.position.clone(),
    );
    let joint_stiffness = serialized_component_batch_f32(
        "yggdrasil.components.JointStiffness",
        control_msg.stiffness.clone(),
    );

    ctx.log_with_cycle(
        "nao/control_message",
        *cycle,
        &[joint_positions, joint_stiffness],
    );
}
