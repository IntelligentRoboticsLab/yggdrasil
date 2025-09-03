use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use rerun::{
    AsComponents, Rotation3D,
    components::{RotationAxisAngle, RotationQuat},
};

use bevy::prelude::*;

use filter::{CovarianceMatrix, KalmanFilter};
use nalgebra::{
    Matrix2, Matrix2x4, Matrix4, Matrix4x2, Point2, Rotation3, Unit, UnitVector3, Vector2, Vector3,
    Vector4, matrix, vector,
};
use rerun::external::arrow;

use crate::{
    core::debug::DebugContext,
    localization::{RobotPose, odometry::Odometry},
    nao::Cycle,
};

const MAX_CONCURRENT_HYPOTHESES: usize = 12;
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
                    set_best_ball,
                )
                    .chain(),
            )
            .add_systems(PostUpdate, log_3d_balls);
    }
}

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
}

impl MovingBallKf {
    pub fn predict(
        &mut self,
        odometry: &Odometry,
        velocity_decay: f32,
        dt: Duration,
        process_noise: CovarianceMatrix<4>,
    ) {
        let dt = dt.as_secs_f32();
        let constant_velocity_prediction = matrix![
            1.0, 0.0, dt, 0.0;
            0.0, 1.0, 0.0, dt;
            0.0, 0.0, velocity_decay, 0.0;
            0.0, 0.0, 0.0, velocity_decay;
        ];

        let inverse_odometry = odometry.offset_to_last.inverse();

        let translation = inverse_odometry.translation;
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
            Matrix4x2::identity(),
            translation.vector,
            process_noise,
        );
    }
}

// #[derive(Clone, Debug)]
// pub struct StationaryBall {
//     pub position: Point2<f32>,
// }

// impl From<Point2<f32>> for StationaryBall {
//     fn from(position: Point2<f32>) -> Self {
//         Self { position }
//     }
// }

#[derive(Clone, Debug)]
pub struct MovingBall {
    pub position: Point2<f32>,
    pub velocity: Vector2<f32>,
}

#[derive(Clone, Component, Debug)]
pub struct BallPerception {
    // Relative position from the robot at which a ball is detected
    pub position: Point2<f32>,
    pub cycle: Cycle,
}

#[derive(Clone, Component, Debug)]
pub struct BallHypothesis {
    pub filter: MovingBallKf,
    pub nll_of_measurements: f32,
    pub nll_weight: f32,
    pub num_observations: u32,
    pub last_update: Instant,
    pub last_cycle: Cycle,
}

impl BallHypothesis {
    #[must_use]
    pub fn is_moving(&self) -> bool {
        const EPSILON: f32 = 0.01;
        self.filter.state().velocity.norm() > EPSILON
    }

    #[must_use]
    pub fn position(&self) -> Point2<f32> {
        self.filter.state().position
    }

    pub fn predict(
        &mut self,
        odometry: &Odometry,
        moving_process_noise: Matrix4<f32>,
        velocity_decay: f32,
        dt: Duration,
    ) {
        // TODO: config values
        const MIN_SPEED: f32 = 0.05; // m/s

        self.filter
            .predict(odometry, velocity_decay, dt, moving_process_noise);

        let pos = self.filter.state().position;
        let cov_2d = self
            .filter
            .covariance()
            .fixed_view::<2, 2>(0, 0)
            .into_owned();

        let gain = nll_of_position(
            pos.coords,
            cov_2d,
            odometry.offset_to_last.translation.vector,
        );
        self.nll_of_measurements += gain;
        self.nll_weight = self.nll_of_measurements + nll_of_mean(cov_2d);

        // demote to stationary
        if (self.num_observations < 5 && self.last_update.elapsed() > Duration::from_secs(3))
            || (self.num_observations > 5 && self.filter.state().velocity.norm() < MIN_SPEED)
        {
            // set velocity to zero
            self.filter.state = Vector4::new(pos.x, pos.y, 0.0, 0.0);
            // TODO: covariance?
        }
    }

    pub fn merge(&mut self, other: &Self) {
        self.nll_of_measurements = self.nll_of_measurements.min(other.nll_of_measurements);
        self.nll_weight = self.nll_weight.min(other.nll_weight);
        self.num_observations += other.num_observations;
        self.last_update = self.last_update.max(other.last_update);
        self.last_cycle = self.last_cycle.max(other.last_cycle);

        self.filter
            .update(
                other.filter.state().position.coords,
                Matrix2x4::identity(),
                other
                    .filter
                    .covariance()
                    .fixed_view::<2, 2>(0, 0)
                    .into_owned(),
            )
            .expect("Failed to merge ball filter");
    }
}

fn predict(mut hypotheses: Query<&mut BallHypothesis>, odometry: Res<Odometry>, time: Res<Time>) {
    for mut hypothesis in &mut hypotheses {
        const VELOCITY_DECAY: f32 = 0.998; // TODO: config value

        let dt = time.delta();

        hypothesis.predict(&odometry, Matrix4::identity(), VELOCITY_DECAY, dt);
    }
}

fn measurement_update(
    mut commands: Commands,
    mut hypotheses: Query<&mut BallHypothesis>,
    measurements: Query<(Entity, &BallPerception), Added<BallPerception>>,
) {
    let amount_of_measurements = measurements.iter().count();

    for (entity, measurement) in &measurements {
        let updated = hypotheses
            .iter_mut()
            .filter(|hypothesis| {
                // TODO: config values
                const MAX_DISTANCE: f32 = 2.5;

                nalgebra::distance(&hypothesis.position(), &measurement.position) < MAX_DISTANCE
            })
            .map(|mut hypothesis| {
                hypothesis.num_observations += 1;
                hypothesis.last_cycle = measurement.cycle;
                hypothesis.last_update = Instant::now();

                // TODO: config values
                let measurement_noise = Matrix2::identity();

                // TODO: use ball distance in noise calculation
                hypothesis
                    .filter
                    .update(
                        measurement.position.coords,
                        Matrix2x4::identity(),
                        measurement_noise,
                    )
                    .expect("Failed to update moving ball filter");
            })
            .count();

        if updated == 0 && amount_of_measurements < MAX_CONCURRENT_HYPOTHESES {
            // TODO: config values
            let initial_measurement_covariance = Matrix4::identity();
            let filter = MovingBallKf::new(measurement.position, initial_measurement_covariance);

            commands.spawn(BallHypothesis {
                filter,
                nll_of_measurements: INITIAL_NLL_OF_MEASUREMENTS,
                nll_weight: INITIAL_NLL_WEIGHT,
                num_observations: 1,
                last_update: Instant::now(),
                last_cycle: measurement.cycle,
            });
        }

        // clean up old perceptions
        commands.entity(entity).despawn();
    }
}

fn merge_balls(mut commands: Commands, mut hypotheses: Query<(Entity, &mut BallHypothesis)>) {
    let mut skip = vec![];
    let mut combinations = hypotheses.iter_combinations_mut();
    while let Some([(entity_a, mut a), (entity_b, b)]) = combinations.fetch_next() {
        if skip.contains(&entity_a) || skip.contains(&entity_b) {
            continue;
        }

        if a.is_moving() != b.is_moving() {
            continue;
        }

        if nalgebra::distance(&a.position(), &b.position()) < 0.1 {
            // Merge the two hypotheses
            a.merge(&b);
            skip.push(entity_b);
        }
    }

    for entity in skip {
        commands.entity(entity).despawn();
    }
}

#[derive(Clone, Copy, Debug, Default, Resource)]
pub struct Ball {
    pub cycle: Cycle,
    pub covariance: Matrix2<f32>,
    pub position: Point2<f32>,
    pub velocity: Option<Vector2<f32>>,
}

fn set_best_ball(hypotheses: Query<&BallHypothesis>, mut ball: ResMut<Ball>) {
    let best_ball = hypotheses
        .iter()
        .min_by(|a, b| a.nll_of_measurements.total_cmp(&b.nll_of_measurements));

    if let Some(best_ball) = best_ball {
        ball.cycle = best_ball.last_cycle;
        ball.position = best_ball.position();

        ball.velocity = best_ball
            .is_moving()
            .then(|| best_ball.filter.state().velocity);
        ball.covariance = best_ball
            .filter
            .covariance
            .fixed_view::<2, 2>(0, 0)
            .into_owned();
    }
}

fn clean_old_hypotheses(mut commands: Commands, hypotheses: Query<(Entity, &BallHypothesis)>) {
    const DESPAWN_TIME: Duration = Duration::from_secs(5);

    for (entity, hypothesis) in &hypotheses {
        // todo dont do time
        if hypothesis.last_update.elapsed() > DESPAWN_TIME {
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
    hypotheses: Query<&BallHypothesis>,
    // local ball rotation
    mut current_rotation: Local<Rotation3<f32>>,
) {
    let pos = robot_pose.robot_to_world(&ball.position);

    let (velocity_vector, velocity_magnitude) = if let Some(velocity_vector) = ball.velocity {
        let velocity_magnitude = velocity_vector.norm();
        (velocity_vector, velocity_magnitude)
    } else {
        (Vector2::default(), 0.0)
    };

    // add a little rotation in the direction of velocity angle rotated by robot orientation
    let ball_radius = 0.05; // 5 cm
    let dt = 1.0 / 82.0; // 82 Hz
    // rotate around normal
    let delta_rotation = Rotation3::from_axis_angle(
        &UnitVector3::new_normalize(Vector3::new(velocity_vector.x, velocity_vector.y, 0.0)),
        velocity_magnitude * dt / ball_radius,
    );

    // update current rotation so that the new rotation is added to it
    *current_rotation = delta_rotation * *current_rotation;

    let rotation = *current_rotation
        * Rotation3::from_axis_angle(&Vector3::z_axis(), robot_pose.world_rotation());

    if hypotheses.is_empty() {
        dbg.log(
            "balls/best",
            &[
                rerun::Transform3D::from_scale((0., 0., 0.)).as_serialized_batches(),
                rerun::Arrows3D::update_fields()
                    .with_colors([(255, 0, 0)])
                    .as_serialized_batches(),
            ],
        );
    } else {
        let max_variance = ball.covariance.diagonal().max();
        let max_std = max_variance.sqrt();

        let (axis, angle) = rotation.axis_angle().unwrap();

        dbg.log_with_cycle(
            "balls/best",
            *cycle,
            &[
                rerun::Transform3D::from_translation((pos.coords.x, pos.coords.y, 0.05))
                    .as_serialized_batches(),
                rerun::SerializedComponentBatch::new(
                    Arc::new(arrow::array::Float32Array::from_value(max_std, 1)),
                    rerun::ComponentDescriptor::new("yggdrasil.components.MaxStd"),
                )
                .as_serialized_batches(),
                rerun::Arrows3D::from_vectors([(velocity_vector.y, -velocity_vector.x, 0.0)])
                    .as_serialized_batches(),
                rerun::SerializedComponentBatch::new(
                    Arc::new(arrow::array::UInt64Array::from_value(
                        ball.cycle.0 as u64,
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
            &rerun::Transform3D::from_rotation(Rotation3D::AxisAngle(RotationAxisAngle::new(
                (axis.x, axis.y, axis.z),
                angle,
            )))
            .as_serialized_batches(),
        );
    }
}

/// Returns the unnormalized negative log-likelihood (nll) of a 2D Gaussian at a given position.
fn nll_of_position(mean: Vector2<f32>, cov: Matrix2<f32>, pos: Vector2<f32>) -> f32 {
    let diff = pos - mean;
    let mahalanobis_distance_sqr = diff.dot(
        &(cov
            .try_inverse()
            .expect("Failed to invert covariance matrix")
            * diff),
    );
    0.5 * mahalanobis_distance_sqr
}

/// Returns the nll of a 2d gaussian at its mean
fn nll_of_mean(cov: Matrix2<f32>) -> f32 {
    0.5 * cov.determinant().max(0.0).ln()
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
