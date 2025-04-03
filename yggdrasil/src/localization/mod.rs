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
    filter_hypotheses, line_update, odometry_update, reset_hypotheses, HypothesisConfig,
    RobotPoseHypothesis,
};
use odal::Config;
use odometry::OdometryConfig;
use pose::initial_pose;
pub use pose::RobotPose;

use rerun::{components::RotationAxisAngle, Rotation3D, TimeColumn};
use serde::{Deserialize, Serialize};

use crate::{
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
                        .run_if(not(is_penalized)),
                    filter_hypotheses,
                    reset_hypotheses,
                )
                    .chain()
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

fn setup_pose_visualization(dbg: DebugContext) {
    let times = TimeColumn::new_sequence("cycle", [0]);
    let color_and_shape = rerun::Boxes3D::update_fields()
        .with_half_sizes([(0.075, 0.1375, 0.2865)])
        .with_colors([rerun::Color::from_rgb(0, 120, 255)])
        .columns_of_unit_batches()
        .expect("failed to create pose visualization");

    let transform = rerun::Transform3D::update_fields()
        .with_axis_length(0.3)
        .columns_of_unit_batches()
        .expect("failed to create view coordinates for pose visualation");

    dbg.send_columns(
        "localization/pose",
        [times],
        color_and_shape.chain(transform),
    );
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
