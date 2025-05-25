use std::time::{Duration, Instant};

use bevy::prelude::*;

use filter::{CovarianceMatrix, KalmanFilter};
use nalgebra::{Matrix2, Matrix2x4, Matrix4, Matrix4x2, Point2, Vector2, Vector4, matrix};

use crate::{localization::odometry::Odometry, nao::Cycle};

const MAX_CONCURRENT_HYPOTHESES: usize = 12;

pub struct BallHypothesisPlugin;

impl Plugin for BallHypothesisPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                predict,
                measurement_update,
                merge_balls,
                clean_old_hypotheses,
                visualize_balls,
            )
                .chain(),
        );
    }
}

#[derive(Clone, Debug, Deref, DerefMut)]
pub struct StationaryBallKf(KalmanFilter<2, StationaryBall>);

impl StationaryBallKf {
    #[must_use]
    pub fn new(initial_position: Point2<f32>, initial_covariance: CovarianceMatrix<2>) -> Self {
        Self(KalmanFilter::new(
            initial_position.into(),
            initial_covariance,
        ))
    }

    pub fn predict(&mut self, odometry: &Odometry, process_noise: CovarianceMatrix<2>) {
        let inverse_odometry = odometry.offset_to_last.inverse();

        let translation = inverse_odometry.translation;
        let rotation = inverse_odometry.rotation.to_rotation_matrix();

        self.0.predict(
            *rotation.matrix(),
            Matrix2::identity(),
            translation.vector,
            process_noise,
        );
    }
}

#[derive(Clone, Debug, Deref, DerefMut)]
pub struct MovingBallKf(KalmanFilter<4, MovingBall>);

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

#[derive(Clone, Debug)]
pub struct StationaryBall {
    pub position: Point2<f32>,
}

impl From<Point2<f32>> for StationaryBall {
    fn from(position: Point2<f32>) -> Self {
        Self { position }
    }
}

#[derive(Clone, Debug)]
pub struct MovingBall {
    pub position: Point2<f32>,
    pub velocity: Vector2<f32>,
}

#[derive(Clone, Debug)]
pub enum BallFilter {
    Stationary(StationaryBallKf),
    Moving(MovingBallKf),
}

impl BallFilter {
    pub fn position(&self) -> Point2<f32> {
        match self {
            BallFilter::Stationary(kf) => kf.state().position,
            BallFilter::Moving(kf) => kf.state().position,
        }
    }
}

#[derive(Clone, Component, Debug)]
pub struct BallPerception {
    // Relative position from the robot at which a ball is detected
    pub position: Point2<f32>,
    pub cycle: Cycle,
}

#[derive(Clone, Component, Debug)]
pub struct BallHypothesis {
    pub filter: BallFilter,
    pub num_observations: u32,
    pub last_update: Instant,
    pub last_cycle: Cycle,
}

impl BallHypothesis {
    pub fn predict(
        &mut self,
        odometry: &Odometry,
        resting_process_noise: Matrix2<f32>,
        moving_process_noise: Matrix4<f32>,
        velocity_decay: f32,
        resting_log_likelihood_threshold: f32,
        dt: Duration,
    ) {
        match &mut self.filter {
            BallFilter::Stationary(resting) => {
                resting.predict(odometry, resting_process_noise);
            }
            // Demote to stationary if it is no longer moving
            BallFilter::Moving(moving) => {
                moving.predict(odometry, velocity_decay, dt, moving_process_noise);

                let velocity_covariance = moving.covariance().fixed_view::<2, 2>(2, 2).into_owned();
                let velocity =
                    nalgebra::vector![moving.state().velocity.x, moving.state().velocity.y];

                let exponent = -velocity.dot(
                    &velocity_covariance
                        .cholesky()
                        .expect("covariance not invertible")
                        .solve(&velocity),
                ) / 2.0;
                let determinant = velocity_covariance.determinant();

                let resting_log_likelihood =
                    exponent - (2.0 * std::f32::consts::PI * determinant.sqrt()).ln();

                if resting_log_likelihood > resting_log_likelihood_threshold {
                    self.filter = BallFilter::Stationary(StationaryBallKf::new(
                        moving.state().position,
                        moving.covariance().fixed_view::<2, 2>(0, 0).into_owned(),
                    ));
                }
            }
        }
    }

    pub fn merge(&mut self, other: &Self) {
        match (&mut self.filter, &other.filter) {
            (BallFilter::Stationary(first), BallFilter::Stationary(second)) => {
                first
                    .update(second.state, Matrix2::identity(), second.covariance())
                    .expect("Failed to update stationary ball filter");
            }
            (BallFilter::Moving(first), BallFilter::Moving(second)) => {
                first
                    .update(second.state, Matrix4::identity(), second.covariance())
                    .expect("Failed to update moving ball filter");
            }
            // no merge
            _ => (),
        };
    }
}

fn predict(mut hypotheses: Query<&mut BallHypothesis>, odometry: Res<Odometry>) {
    for mut hypothesis in &mut hypotheses {
        let dt = hypothesis.last_update.elapsed();

        hypothesis.predict(
            &odometry,
            Matrix2::identity(),
            Matrix4::identity(),
            0.9,
            0.5,
            dt,
        );
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
                const MAX_DISTANCE: f32 = 1.0;

                nalgebra::distance(&hypothesis.filter.position(), &measurement.position)
                    < MAX_DISTANCE
            })
            .map(|mut hypothesis| {
                hypothesis.num_observations += 1;
                hypothesis.last_update = Instant::now();

                match hypothesis.filter {
                    BallFilter::Stationary(ref mut kf) => {
                        // TODO: config values
                        let measurement_noise = Matrix2::identity();

                        // TODO: use ball distance in noise calculation
                        kf.update(
                            measurement.position.coords,
                            Matrix2::identity(),
                            measurement_noise,
                        )
                        .expect("Failed to update stationary ball filter");
                    }
                    BallFilter::Moving(ref mut kf) => {
                        // TODO: config values
                        let measurement_noise = Matrix2::identity();

                        // TODO: use ball distance in noise calculation
                        kf.update(
                            measurement.position.coords,
                            Matrix2x4::identity(),
                            measurement_noise,
                        )
                        .expect("Failed to update moving ball filter");
                    }
                }
            })
            .count();

        if updated == 0 && amount_of_measurements < MAX_CONCURRENT_HYPOTHESES {
            // TODO: config values
            let initial_measurement_covariance = Matrix2::identity();

            let filter = BallFilter::Stationary(StationaryBallKf::new(
                measurement.position,
                initial_measurement_covariance,
            ));

            // TODO: use ball distance in noise calculation
            commands.spawn(BallHypothesis {
                filter,
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

        if nalgebra::distance(&a.filter.position(), &b.filter.position()) < 0.1 {
            // Merge the two hypotheses
            a.merge(&b);
        }

        skip.push(entity_b);
    }

    for entity in skip {
        commands.entity(entity).despawn();
    }
}

fn clean_old_hypotheses(mut commands: Commands, hypotheses: Query<(Entity, &BallHypothesis)>) {
    for (entity, hypothesis) in &hypotheses {
        // todo dont do time
        if hypothesis.last_update.elapsed() > Duration::from_secs(5) {
            commands.entity(entity).despawn();
        }
    }
}

fn visualize_balls(hypotheses: Query<&BallHypothesis>) {
    for hypothesis in &hypotheses {
        println!("Hypothesis: {:?}", hypothesis);
    }
    println!();
}

impl From<Vector2<f32>> for StationaryBall {
    fn from(position: Vector2<f32>) -> Self {
        Self {
            position: Point2::from(position),
        }
    }
}

impl From<StationaryBall> for Vector2<f32> {
    fn from(ball: StationaryBall) -> Self {
        ball.position.coords
    }
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
