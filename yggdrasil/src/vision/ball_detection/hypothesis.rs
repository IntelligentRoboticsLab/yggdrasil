use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use rerun::{AsComponents, Rotation3D, components::RotationAxisAngle};

use bevy::prelude::*;

use filter::{CovarianceMatrix, KalmanFilter, mahalanobis_distance};
use nalgebra::{
    Matrix2, Matrix2x4, Matrix4, Point2, Rotation3, UnitVector3, Vector2, Vector3, Vector4, matrix,
    vector,
};
use rerun::external::arrow;
use serde::{Deserialize, Serialize};

use crate::{
    core::debug::{DebugContext, serialized_component_batch_f32},
    localization::{RobotPose, odometry::Odometry},
    nao::Cycle,
    vision::ball_detection::classifier::BallPerception,
};

#[derive(Debug, Clone, Default, Resource, Serialize, Deserialize)]
pub struct BallHypothesisConfig {
    /// Maximum amount of ball hypotheses tracked at the same time
    pub max_concurrent_hypotheses: u32,

    /// Minimum speed to be considered as still moving
    pub min_moving_speed: f32,

    /// Minimum amount of observations before a moving ball can be demoted to stationary for losing speed
    pub min_demotion_observations: u32,

    /// Speed loss in m/s
    pub linear_velocity_decay: f32,

    /// Speed loss in percentage/s
    pub exponential_velocity_decay: f32,

    /// Maximum mahalanobis distance to associate a measurement to a hypothesis
    pub max_mahalonobis_association_distance: f32,

    /// Maximum euclidean distance to associate a measurement to a hypothesis
    pub max_euclidean_association_distance: f32,

    /// Maximum speed difference for merging moving ball hypotheses in m/s
    pub max_merge_speed_difference: f32,

    /// Maximum angle difference for merging moving ball hypotheses in radians
    pub max_merge_angle_difference: f32,

    /// Uncertainty in meters for despawning a ball hypothesis
    pub despawn_uncertainty: f32,

    /// Process noise for moving ball model
    pub moving_process_noise: [f32; 4],

    /// Ball position measurement noise
    pub measurement_noise: [f32; 2],

    /// Initial covariance for new ball hypotheses
    pub initial_covariance: [f32; 4],
}

/// Ball radius in meters for logging ball rotation
const BALL_RADIUS: f32 = 0.05;

/// Minimum lifetime of a ball hypothesis before it can be removed
const MIN_BALL_LIFETIME: Duration = Duration::from_secs(3);

/// Duration after which an unreliable ball hypothesis can be demoted to stationary
///
/// Only applies if the ball has less than `min_demotion_observations` observations
const UNRELIABLE_DEMOTION_DURATION: Duration = Duration::from_secs(3);

const INITIAL_NLL_OF_MEASUREMENTS: f32 = 0.0;
const INITIAL_NLL_WEIGHT: f32 = f32::MAX;

pub struct BallHypothesisPlugin;

impl Plugin for BallHypothesisPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Ball>()
            .add_systems(PostStartup, setup_3d_ball_debug_logging)
            .add_systems(
                Update,
                (
                    predict,
                    measurement_update,
                    merge_balls,
                    clean_old_hypotheses,
                    normalize_measurement_nll,
                    update_reliable_ball,
                )
                    .chain(),
            )
            .add_systems(PostUpdate, log_3d_balls);
    }
}

/// Kalman filter for a moving ball with state [x, y, vx, vy]
#[derive(Clone, Debug, Deref, DerefMut)]
pub struct MovingBallKf(KalmanFilter<4, MovingBall>);

impl MovingBallKf {
    #[must_use]
    pub fn new(initial_position: Point2<f32>, initial_covariance: CovarianceMatrix<4>) -> Self {
        Self(KalmanFilter::new(
            nalgebra::vector![initial_position.x, initial_position.y, 0.0, 0.0].into(),
            initial_covariance,
        ))
    }

    #[must_use]
    pub fn is_moving(&self) -> bool {
        const EPSILON: f32 = 0.01;
        self.0.state().velocity.norm() > EPSILON
    }

    pub fn predict(
        &mut self,
        odometry: &Odometry,
        linear_velocity_decay: f32,
        exponential_velocity_decay: f32,
        dt: Duration,
        process_noise: CovarianceMatrix<4>,
    ) {
        let dt = dt.as_secs_f32();
        let constant_velocity_prediction = matrix![
            1.0, 0.0, dt, 0.0;
            0.0, 1.0, 0.0, dt;
            0.0, 0.0, 1.0 - exponential_velocity_decay * dt, 0.0;
            0.0, 0.0, 0.0, 1.0 - exponential_velocity_decay * dt;
        ];

        let inverse_odometry = odometry.offset_to_last.inverse();

        let translation = inverse_odometry.translation;

        // apply linear velocity decay
        let linear_decay = if self.is_moving() {
            -self.state().velocity.normalize()
                * (dt * linear_velocity_decay).min(self.state().velocity.norm())
        } else {
            Vector2::zeros()
        };

        let control_vector = vector![translation.x, translation.y, linear_decay.x, linear_decay.y];

        let rotation = inverse_odometry.rotation.to_rotation_matrix();
        let rot_mat = rotation.matrix();

        let state_rotation = matrix![
            rot_mat.m11, rot_mat.m12, 0.0, 0.0;
            rot_mat.m21, rot_mat.m22, 0.0, 0.0;
            0.0, 0.0, rot_mat.m11, rot_mat.m12;
            0.0, 0.0, rot_mat.m21, rot_mat.m22;
        ];

        let state_transition_model = constant_velocity_prediction * state_rotation;

        self.0.predict(
            state_transition_model,
            Matrix4::identity(),
            control_vector,
            process_noise,
        );
    }
}

#[derive(Clone, Debug)]
pub struct MovingBall {
    pub position: Point2<f32>,
    pub velocity: Vector2<f32>,
}

#[derive(Clone, Component, Debug)]
pub struct BallHypothesis {
    pub filter: MovingBallKf,
    pub nll_of_measurements: f32,
    pub nll_weight: f32,
    pub num_observations: u32,
    pub spawned_at: Instant,
    pub last_update: Instant,
    pub last_cycle: Cycle,
}

impl BallHypothesis {
    #[must_use]
    pub fn is_moving(&self) -> bool {
        self.filter.is_moving()
    }

    #[must_use]
    pub fn position(&self) -> Point2<f32> {
        self.filter.state().position
    }

    pub fn predict(&mut self, odometry: &Odometry, dt: Duration, config: &BallHypothesisConfig) {
        let moving_process_noise =
            Matrix4::from_diagonal(&Vector4::from(config.moving_process_noise));

        self.filter.predict(
            odometry,
            config.linear_velocity_decay,
            config.exponential_velocity_decay,
            dt,
            moving_process_noise,
        );

        let is_reliable = self.num_observations >= config.min_demotion_observations;

        // demote to stationary
        if (!is_reliable && self.last_update.elapsed() > UNRELIABLE_DEMOTION_DURATION)
            || (is_reliable && self.filter.state().velocity.norm() < config.min_moving_speed)
        {
            let pos = self.filter.state().position;
            // set velocity to zero
            self.filter.state = Vector4::new(pos.x, pos.y, 0.0, 0.0);
        }
    }

    pub fn merge(&mut self, other: &Self) {
        self.nll_of_measurements = self.nll_of_measurements.min(other.nll_of_measurements);
        self.nll_weight = self.nll_weight.min(other.nll_weight);
        self.num_observations += other.num_observations;
        self.last_update = self.last_update.max(other.last_update);
        self.last_cycle = self.last_cycle.max(other.last_cycle);

        if self
            .filter
            .update(
                other.filter.state().position.coords,
                Matrix2x4::identity(),
                other
                    .filter
                    .covariance()
                    .fixed_view::<2, 2>(0, 0)
                    .into_owned(),
            )
            .is_err()
        {
            tracing::warn!("Failed to merge ball filter");
        }
    }
}

fn predict(
    mut hypotheses: Query<&mut BallHypothesis>,
    odometry: Res<Odometry>,
    time: Res<Time>,
    config: Res<BallHypothesisConfig>,
) {
    for mut hypothesis in &mut hypotheses {
        let dt = time.delta();

        hypothesis.predict(&odometry, dt, &config);
    }
}

fn measurement_update(
    mut commands: Commands,
    mut hypotheses: Query<&mut BallHypothesis>,
    measurements: Query<(Entity, &BallPerception), Added<BallPerception>>,
    config: Res<BallHypothesisConfig>,
) {
    for (entity, measurement) in &measurements {
        let amount_of_hypotheses = hypotheses.iter().count();

        let updated = hypotheses
            .iter_mut()
            .filter(|hypothesis| {
                mahalanobis_distance(
                    measurement.position.coords,
                    hypothesis.position().coords,
                    hypothesis
                        .filter
                        .covariance()
                        .fixed_view::<2, 2>(0, 0)
                        .into_owned(),
                )
                .unwrap_or_default()
                    < config.max_mahalonobis_association_distance
                    && measurement.position.coords.norm()
                        < config.max_euclidean_association_distance
            })
            .map(|mut hypothesis| {
                hypothesis.num_observations += 1;
                hypothesis.last_cycle = measurement.cycle;
                hypothesis.last_update = Instant::now();

                // use measured ball distance to robot in noise calculation
                // (further away ball projections will be more noisy)
                let measurement_noise = Matrix2::from_diagonal(
                    &(Vector2::from(config.measurement_noise)
                        * noise_scale(measurement.position.coords.norm())),
                );

                // update nll
                let position = hypothesis.filter.state().position;
                let cov_2d = hypothesis
                    .filter
                    .covariance()
                    .fixed_view::<2, 2>(0, 0)
                    .into_owned();

                if let Some(gain) =
                    nll_of_position(measurement.position, measurement_noise, position)
                {
                    hypothesis.nll_of_measurements += gain;
                    hypothesis.nll_weight = hypothesis.nll_of_measurements - nll_of_mean(cov_2d);
                } else {
                    tracing::warn!("Failed to calculate nll for ball hypothesis");
                }

                // update filter
                if hypothesis
                    .filter
                    .update(
                        measurement.position.coords,
                        Matrix2x4::identity(),
                        measurement_noise,
                    )
                    .is_err()
                {
                    tracing::warn!("Failed to update moving ball filter");
                }
            })
            .count();

        if updated == 0 && amount_of_hypotheses < config.max_concurrent_hypotheses as usize {
            let initial_covariance =
                Matrix4::from_diagonal(&Vector4::from(config.initial_covariance));

            let filter = MovingBallKf::new(measurement.position, initial_covariance);

            commands.spawn(BallHypothesis {
                filter,
                nll_of_measurements: INITIAL_NLL_OF_MEASUREMENTS,
                nll_weight: INITIAL_NLL_WEIGHT,
                num_observations: 1,
                spawned_at: Instant::now(),
                last_update: Instant::now(),
                last_cycle: measurement.cycle,
            });
        }

        // clean up old perceptions
        commands.entity(entity).despawn();
    }
}

/// Merges ball hypotheses that are of the same type (stationary/moving) and close to each other
fn merge_balls(
    mut commands: Commands,
    mut hypotheses: Query<(Entity, &mut BallHypothesis)>,
    config: Res<BallHypothesisConfig>,
) {
    let mut skip = vec![];
    let mut combinations = hypotheses.iter_combinations_mut();
    while let Some([(entity_a, mut a), (entity_b, b)]) = combinations.fetch_next() {
        if skip.contains(&entity_a) || skip.contains(&entity_b) {
            continue;
        }

        // only merge if they are
        // a) both stationary
        // b) both moving with similar velocity and direction
        if a.is_moving() != b.is_moving()
            || (a.is_moving()
                && (a.filter.state().velocity - b.filter.state().velocity).norm()
                    > config.max_merge_speed_difference
                && a.filter
                    .state()
                    .velocity
                    .angle(&b.filter.state().velocity)
                    .abs()
                    > config.max_merge_angle_difference)
        {
            continue;
        }

        // only merge if they are close enough (considering uncertainty)
        if mahalanobis_distance(
            a.position().coords,
            b.position().coords,
            a.filter.covariance().fixed_view::<2, 2>(0, 0).into_owned(),
        )
        .is_ok_and(|d| d < config.max_mahalonobis_association_distance)
        {
            // Merge the two hypotheses
            a.merge(&b);
            skip.push(entity_b);
        }
    }

    for entity in skip {
        commands.entity(entity).despawn();
    }
}

#[derive(Clone, Debug)]
pub struct BallState {
    pub last_cycle: Cycle,
    pub last_update: Instant,
    covariance: Matrix4<f32>,
    pub position: Point2<f32>,
    pub velocity: Option<Vector2<f32>>,
}

#[derive(Clone, Debug, Default, Resource)]
pub enum Ball {
    Some(BallState),
    #[default]
    None,
}

impl Ball {
    #[must_use]
    pub fn as_option(&self) -> Option<&BallState> {
        match self {
            Ball::Some(ball_state) => Some(ball_state),
            Ball::None => None,
        }
    }
}

fn update_reliable_ball(hypotheses: Query<&BallHypothesis>, mut ball: ResMut<Ball>) {
    let Some(reliable_ball) = hypotheses
        .iter()
        .min_by(|a, b| a.nll_of_measurements.total_cmp(&b.nll_of_measurements))
    else {
        *ball = Ball::None;
        return;
    };

    *ball = Ball::Some(BallState {
        last_cycle: reliable_ball.last_cycle,
        last_update: reliable_ball.last_update,
        covariance: reliable_ball.filter.covariance(),
        position: reliable_ball.position(),
        velocity: reliable_ball
            .is_moving()
            .then(|| reliable_ball.filter.state().velocity),
    });
}

fn clean_old_hypotheses(
    mut commands: Commands,
    hypotheses: Query<(Entity, &BallHypothesis)>,
    config: Res<BallHypothesisConfig>,
) {
    for (entity, hypothesis) in &hypotheses {
        if hypothesis.spawned_at.elapsed() < MIN_BALL_LIFETIME {
            continue;
        }

        if hypothesis
            .filter
            .covariance()
            .fixed_view::<2, 2>(0, 0)
            .diagonal()
            .max()
            .sqrt()
            > config.despawn_uncertainty
        {
            commands.entity(entity).despawn();
        }
    }
}

fn normalize_measurement_nll(mut hypotheses: Query<&mut BallHypothesis>) {
    let lowest = hypotheses
        .iter()
        .map(|hypothesis| hypothesis.nll_of_measurements)
        .min_by(f32::total_cmp);

    if let Some(lowest) = lowest {
        for mut hypothesis in &mut hypotheses {
            hypothesis.nll_of_measurements -= lowest;
        }
    }
}

fn setup_3d_ball_debug_logging(dbg: DebugContext) {
    dbg.log_static(
        "balls/best/mesh",
        &[rerun::Asset3D::from_file_path("./assets/rerun/ball.glb")
            .expect("failed to load ball model")
            .with_media_type(rerun::MediaType::glb())
            .as_serialized_batches()],
    );

    dbg.log_static(
        "balls/best",
        &[rerun::Arrows3D::update_fields()
            .with_radii(std::iter::once(rerun::components::Radius::new_ui_points(
                1.5,
            )))
            .as_serialized_batches()],
    );

    dbg.log_with_cycle(
        "balls/best",
        Cycle::default(),
        &rerun::Transform3D::from_scale((0., 0., 0.)),
    );
}

fn log_3d_balls(
    dbg: DebugContext,
    ball: Res<Ball>,
    cycle: Res<Cycle>,
    robot_pose: Res<RobotPose>,
    time: Res<Time>,
    // local ball rotation
    mut current_rotation: Local<Rotation3<f32>>,
) {
    let Ball::Some(ref ball) = *ball else {
        dbg.log(
            "balls/best",
            &[
                rerun::Transform3D::from_scale((0., 0., 0.)).as_serialized_batches(),
                rerun::Arrows3D::update_fields()
                    .with_colors([(255, 0, 0)])
                    .as_serialized_batches(),
            ],
        );
        return;
    };

    let position = robot_pose.robot_to_world(&ball.position);
    let dt = time.delta_secs();

    let (velocity_vector, delta_rotation) = if let Some(velocity_vector) = ball.velocity {
        // rotate the velocity vector to world frame
        let rotation = robot_pose.inner.rotation;
        let velocity_vector = rotation * velocity_vector;

        let velocity_magnitude = velocity_vector.norm();

        // rotate around normal
        let delta_rotation = Rotation3::from_axis_angle(
            &UnitVector3::new_normalize(Vector3::new(velocity_vector.y, -velocity_vector.x, 0.0)),
            -velocity_magnitude * dt / BALL_RADIUS,
        );

        (velocity_vector, delta_rotation)
    } else {
        (Vector2::default(), Rotation3::identity())
    };

    // update current rotation so that the new rotation is added to it
    *current_rotation = delta_rotation * *current_rotation;

    let stds = ball.covariance.diagonal().map(f32::sqrt);
    let (axis, angle) = current_rotation
        .axis_angle()
        .unwrap_or((Vector3::z_axis(), 0.0));

    dbg.log_with_cycle(
        "balls/best",
        *cycle,
        &[
            // ball position
            rerun::Transform3D::from_translation((position.coords.x, position.coords.y, 0.05))
                .as_serialized_batches(),
            // velocity arrow
            rerun::Arrows3D::from_vectors([(velocity_vector.x, velocity_vector.y, 0.0)])
                .as_serialized_batches(),
            serialized_component_batch_f32("yggdrasil.components.StdDevs", stds.iter().copied())
                .as_serialized_batches(),
            rerun::SerializedComponentBatch::new(
                Arc::new(arrow::array::UInt64Array::from_value(
                    ball.last_cycle.0 as u64,
                    1,
                )),
                rerun::ComponentDescriptor::new("yggdrasil.components.BallDetectionCycle"),
            )
            .as_serialized_batches(),
        ],
    );
    dbg.log_with_cycle(
        "balls/best/mesh",
        *cycle,
        // ball rotation is performed separately so that it doesn't affect the velocity arrow
        &rerun::Transform3D::from_rotation(Rotation3D::AxisAngle(RotationAxisAngle::new(
            (axis.x, axis.y, axis.z),
            angle,
        )))
        .as_serialized_batches(),
    );
}

/// Returns the unnormalized negative log-likelihood (nll) of a 2D Gaussian at a given position.
fn nll_of_position(mean: Point2<f32>, cov: Matrix2<f32>, pos: Point2<f32>) -> Option<f32> {
    let diff = pos - mean;
    let mahalanobis_distance_sqr = diff.dot(&(cov.try_inverse()? * diff));
    Some(0.5 * mahalanobis_distance_sqr)
}

/// Returns the nll of a 2d gaussian at its mean
fn nll_of_mean(cov: Matrix2<f32>) -> f32 {
    0.5 * cov.determinant().max(0.0).ln()
}

/// Scales measurement noise based on distance to robot
///
/// See: <https://www.desmos.com/calculator/ijpgmecivz>
fn noise_scale(distance: f32) -> f32 {
    0.5 * distance.powi(2).min(1.0)
}

impl From<Vector4<f32>> for MovingBall {
    fn from(v: Vector4<f32>) -> Self {
        Self {
            position: Point2::new(v.x, v.y),
            velocity: Vector2::new(v.z, v.w),
        }
    }
}

impl From<MovingBall> for Vector4<f32> {
    fn from(ball: MovingBall) -> Self {
        Vector4::new(
            ball.position.x,
            ball.position.y,
            ball.velocity.x,
            ball.velocity.y,
        )
    }
}
