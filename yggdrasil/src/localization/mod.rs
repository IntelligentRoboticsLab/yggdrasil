pub mod correction;
pub mod correspondence;
pub mod hypothesis;
pub mod odometry;
pub mod pose;

use bevy::prelude::*;

use correction::GradientDescentConfig;
use correspondence::CorrespondenceConfig;
use filter::CovarianceMatrix;
use hypothesis::{
    HypothesisConfig, RobotPoseHypothesis, filter_hypotheses, line_update, odometry_update,
    reset_hypotheses,
};
use odal::Config;
use odometry::OdometryConfig;
pub use pose::RobotPose;
use pose::initial_pose;

use rerun::{Rotation3D, TimeColumn, components::RotationAxisAngle};
use serde::{Deserialize, Serialize};

use crate::{
    behavior::primary_state::PrimaryState,
    core::{
        config::{layout::LayoutConfig, showtime::PlayerConfig},
        debug::DebugContext,
    },
    game_controller::penalty::is_penalized,
    motion::{keyframe::KeyframeExecutor, walking_engine::Gait},
    nao::Cycle,
    prelude::ConfigExt,
    sensor::fsr::Contacts,
};

/// The localization plugin provides functionalities related to the localization of the robot.
pub struct LocalizationPlugin;

impl Plugin for LocalizationPlugin {
    fn build(&self, app: &mut App) {
        app.init_config::<LocalizationConfig>()
            .add_plugins(odometry::OdometryPlugin)
            .add_systems(PostStartup, (initialize_pose, setup_pose_visualization))
            .add_systems(
                PreUpdate,
                (
                    (odometry_update, line_update.run_if(not(motion_is_unsafe)))
                        .run_if(not(is_penalized.or(in_pre_walking_state))),
                    filter_hypotheses,
                    reset_hypotheses,
                )
                    .after(odometry::update_odometry),
            )
            .add_systems(PostUpdate, (visualize_pose, visualize_pose_hypotheses));
    }
}

#[derive(Resource, Debug, Clone, Serialize, Deserialize)]
pub struct LocalizationConfig {
    pub odometry: OdometryConfig,
    pub correspondence: CorrespondenceConfig,
    pub hypothesis: HypothesisConfig,
    pub gradient_descent: GradientDescentConfig,
}

impl Config for LocalizationConfig {
    const PATH: &'static str = "localization.toml";
}

fn initialize_pose(
    mut commands: Commands,
    layout: Res<LayoutConfig>,
    player: Res<PlayerConfig>,
    localization: Res<LocalizationConfig>,
) {
    let pose = initial_pose(&layout, player.player_number);

    let hypothesis = RobotPoseHypothesis::new(
        pose,
        CovarianceMatrix::from_diagonal(&localization.hypothesis.variance_initial.into()),
        localization.hypothesis.score_initial,
    );

    commands.spawn(hypothesis);
    commands.insert_resource(pose);
}

fn motion_is_unsafe(
    keyframe_executor: Res<KeyframeExecutor>,
    motion_state: Res<State<Gait>>,
    contacts: Res<Contacts>,
) -> bool {
    keyframe_executor.active_motion.is_some()
        || !matches!(motion_state.get(), Gait::Standing | Gait::Walking)
        || !contacts.ground
}

/// Checks if we are in any of the pre-walking states.
///
/// We assume we start in the perfect position, so we don't need to do any type of localization.
fn in_pre_walking_state(state: Res<PrimaryState>) -> bool {
    matches!(
        state.as_ref(),
        PrimaryState::Sitting | PrimaryState::Standby | PrimaryState::Initial
    )
}

fn setup_pose_visualization(dbg: DebugContext) {
    let times = TimeColumn::new_sequence("cycle", [0]);
    let transform = rerun::Transform3D::update_fields()
        .with_axis_length(0.3)
        .columns_of_unit_batches()
        .expect("failed to create view coordinates for pose visualation");

    dbg.send_columns("localization/pose", [times], transform);
}

fn visualize_pose(dbg: DebugContext, cycle: Res<Cycle>, pose: Res<RobotPose>) {
    let position = pose.world_position();
    dbg.log_with_cycle(
        "localization/pose",
        *cycle,
        &rerun::Transform3D::from_rotation(Rotation3D::AxisAngle(RotationAxisAngle::new(
            (0.0, 0.0, 1.0),
            pose.world_rotation(),
        )))
        .with_translation((position.x, position.y, 0.2865)),
    );
}

fn visualize_pose_hypotheses(
    dbg: DebugContext,
    cycle: Res<Cycle>,
    hypotheses: Query<&RobotPoseHypothesis>,
) {
    dbg.log_with_cycle(
        "localization/hypotheses",
        *cycle,
        &rerun::Arrows3D::from_vectors(hypotheses.iter().map(|hypothesis| {
            let rotation = hypothesis.filter.state().world_rotation();
            (rotation.cos(), rotation.sin(), 0.0)
        }))
        .with_origins(hypotheses.iter().map(|hypothesis| {
            let position = hypothesis.filter.state().world_position();
            (position.x, position.y, 0.1)
        }))
        .with_labels(
            hypotheses
                .iter()
                .map(|hypothesis| format!("{:.2}", hypothesis.score)),
        )
        .with_colors(hypotheses.iter().map(|_| (0, 255, 255))),
    );
}
