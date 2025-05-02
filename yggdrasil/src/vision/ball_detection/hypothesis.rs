use std::time::{Duration, Instant};

use bevy::prelude::*;

use filter::{CovarianceMatrix, KalmanFilter};
use nalgebra::{Matrix2, Matrix2x4, Matrix4, Matrix4x2, Point2, Vector2, Vector4, matrix};

use crate::localization::odometry::Odometry;

#[derive(Clone, Debug)]
pub struct StationaryBallKf(KalmanFilter<2, StationaryBall>);

impl StationaryBallKf {
    #[must_use]
    pub fn new(initial_position: Point2<f32>, initial_covariance: CovarianceMatrix<2>) -> Self {
        Self(KalmanFilter::new(initial_position, initial_covariance))
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

#[derive(Clone, Debug)]
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
            BallFilter::Stationary(kf) => kf.0.state(),
            BallFilter::Moving(kf) => kf.0.state().position,
        }
    }
}

#[derive(Clone, Component, Debug)]
pub struct BallPerception {
    // Relative position from the robot at which a ball is detected
    pub position: Point2<f32>,
}

#[derive(Clone, Component, Debug)]
pub struct BallHypothesis {
    pub filter: BallFilter,
    pub num_observations: u32,
    pub last_observation: Instant,
}

pub fn predict(mut hypotheses: Query<&mut BallHypothesis>, odometry: Res<Odometry>) {
    for mut hypothesis in &mut hypotheses {
        let dt = hypothesis.last_observation.elapsed();

        match &mut hypothesis.filter {
            BallFilter::Stationary(kf) => {
                // TODO: config values
                let process_noise = Matrix2::identity();

                kf.predict(&odometry, process_noise);
            }
            BallFilter::Moving(kf) => {
                // TODO: config values
                let process_noise = Matrix4::identity();
                let velocity_decay = 0.9;

                kf.predict(&odometry, velocity_decay, dt, process_noise);
            }
        }
    }
}

pub fn measurement_update(
    mut commands: Commands,
    mut hypotheses: Query<&mut BallHypothesis>,
    measurements: Query<&BallPerception, Added<BallPerception>>,
) {
    for measurement in &measurements {
        let distance = measurement.position.coords.norm();

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
                hypothesis.last_observation = Instant::now();

                match hypothesis.filter {
                    // TODO: promotion of stationary -> moving filters
                    // and demotion of moving -> stationary filters
                    BallFilter::Stationary(mut kf) => {
                        // TODO: config values
                        let measurement_noise = Matrix2::identity();

                        // TODO: use ball distance in noise calculation
                        kf.0.update(measurement, Matrix2::identity(), measurement_noise);
                    }
                    BallFilter::Moving(mut kf) => {
                        // TODO: config values
                        let measurement_noise = Matrix2::identity();

                        // TODO: use ball distance in noise calculation
                        kf.0.update(measurement, Matrix2x4::identity(), measurement_noise);
                    }
                }
            })
            .count();

        if updated == 0 {
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
                last_observation: Instant::now(),
            });
        }
    }
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
