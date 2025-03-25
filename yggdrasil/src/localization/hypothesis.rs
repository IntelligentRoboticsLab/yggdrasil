use bevy::prelude::*;
use filter::{StateMatrix, StateTransform, StateVector, UnscentedKalmanFilter, WeightVector};
use nalgebra::{point, vector, ComplexField, Point2, UnitComplex};
use num::Complex;

use super::RobotPose;

#[derive(Clone, Component)]
pub struct RobotPoseHypothesis {
    filter: UnscentedKalmanFilter<3, 7, RobotPose>,
    pub score: f32,
}

impl RobotPoseHypothesis {
    /// Mitigate numerical instability with covariance matrix by ensuring it is symmetric
    pub fn fix_covariance(&mut self) {
        let cov = &mut self.filter.covariance;
        *cov = (*cov + cov.transpose()) * 0.5;
    }
}

#[derive(Debug, Clone, Copy)]
struct LineMeasurement {
    distance: f32,
    angle: f32,
}

impl From<StateVector<2>> for LineMeasurement {
    fn from(state: StateVector<2>) -> Self {
        Self {
            distance: state.x,
            angle: state.y,
        }
    }
}

impl From<LineMeasurement> for StateVector<2> {
    fn from(segment: LineMeasurement) -> Self {
        vector![segment.distance, segment.angle]
    }
}

impl StateTransform<2> for LineMeasurement {
    fn into_state_mean<const N: usize>(
        weights: WeightVector<N>,
        states: StateMatrix<2, N>,
    ) -> StateVector<2> {
        let mut mean_distance = 0.0;
        let mut mean_angle = Complex::ZERO;

        for (&weight, pose) in weights.iter().zip(states.column_iter()) {
            mean_distance += weight * pose.x;
            mean_angle += weight * Complex::cis(pose.y);
        }

        vector![mean_distance, mean_angle.argument()]
    }

    fn residual(measurement: StateVector<2>, prediction: StateVector<2>) -> StateVector<2> {
        vector![
            measurement.x - prediction.x,
            (UnitComplex::new(measurement.y) / UnitComplex::new(prediction.y)).angle()
        ]
    }
}

struct CircleMeasurement {
    position: Point2<f32>,
}

impl From<StateVector<2>> for CircleMeasurement {
    fn from(value: StateVector<2>) -> Self {
        CircleMeasurement {
            position: point![value.x, value.y],
        }
    }
}

impl From<CircleMeasurement> for StateVector<2> {
    fn from(value: CircleMeasurement) -> Self {
        vector![value.position.x, value.position.y]
    }
}

impl StateTransform<2> for CircleMeasurement {
    // only uses linear values (no angles), so we can use the default impl
}
