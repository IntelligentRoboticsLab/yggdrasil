use bevy::prelude::*;
use filter::{CovarianceMatrix, StateTransform, StateVector, UnscentedKalmanFilter};
use nalgebra::{point, Point2, Vector2};

use super::classifier::Ball;

// pub struct BallTrackerPlugin;

// impl Plugin for BallTrackerPlugin {
//     fn build(&self, app: &mut App) {
//         app.add_systems(
//             PostUpdate,
//             (
//                 log_ball_classifications::<Top>.run_if(resource_exists_and_changed::<Balls<Top>>),
//                 log_ball_classifications::<Bottom>
//                     .run_if(resource_exists_and_changed::<Balls<Bottom>>),
//             ),
//         );
//     }
// }

enum Hypothesis{
    Moving,
    Stationary,
}

#[derive(Resource, Deref, DerefMut)]
pub struct BallTracker {
    #[deref]
    position_kf: UnscentedKalmanFilter<2, 5, BallPosition>,
    prediction_noise: CovarianceMatrix<2>,
    sensor_noise: CovarianceMatrix<2>,
}

impl BallTracker {
    fn initialize(&mut self) {
        let starting_position = BallPosition(Point2::new(0.0, 0.0));
        let starting_position_cov = nalgebra::SMatrix::<f32, 2, 2>::from_diagonal_element(0.05);
        self.position_kf = UnscentedKalmanFilter::<2, 5, BallPosition>::new(
            starting_position,
            starting_position_cov,
        );
        // Default value for the position update noise
        self.prediction_noise = filter::CovarianceMatrix::from_diagonal_element(0.1);
        self.sensor_noise = filter::CovarianceMatrix::from_diagonal_element(0.001);
    }

    #[inline]
    #[must_use]
    pub fn get_state(&mut self) -> BallPosition {
        self.position_kf.state()
    }

    pub fn predict(&mut self) {
        let f = |p: BallPosition| p;
        self.position_kf.predict(f, self.prediction_noise);
    }

    pub fn update(&mut self, measurement: BallPosition) {
        let h = |p: BallPosition| p;
        self.position_kf.update(h, measurement, self.sensor_noise);
    }
}

#[derive(Deref, DerefMut, Clone, Copy, Resource)]
pub struct BallPosition(pub Point2<f32>);

impl From<BallPosition> for StateVector<2> {
    fn from(value: BallPosition) -> Self {
        value.xy().coords
    }
}

impl From<StateVector<2>> for BallPosition {
    fn from(value: StateVector<2>) -> Self {
        BallPosition(point![value.x, value.y])
    }
}

impl StateTransform<2> for BallPosition {}
