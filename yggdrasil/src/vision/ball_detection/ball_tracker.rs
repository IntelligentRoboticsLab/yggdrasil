use std::time::Instant;

use bevy::prelude::*;
use filter::{CovarianceMatrix, StateTransform, StateVector, UnscentedKalmanFilter};
use nalgebra::{point, Point2};
use rerun::datatypes::Bool;

use crate::nao::Cycle;

pub const STATIONARY_THRESHOLD: f32 = 80.0;

#[derive(Debug)]
pub enum Hypothesis {
    Moving(f32),
    Stationary(f32),
}

#[derive(Resource, Deref, DerefMut)]
pub struct BallTracker {
    #[deref]
    pub position_kf: UnscentedKalmanFilter<2, 5, BallPosition>,
    pub prediction_noise: CovarianceMatrix<2>,
    pub sensor_noise: CovarianceMatrix<2>,
    pub cycle: Cycle,
    pub timestamp: Instant,
}

impl BallTracker {
    #[inline]
    #[must_use]
    pub fn state(&self) -> BallPosition {
        self.position_kf.state()
    }

    #[inline]
    #[must_use]
    pub fn covariance(&self) -> CovarianceMatrix<2> {
        self.position_kf.covariance()
    }

    pub fn cutoff(&self) -> Hypothesis {
        let max_variance = self
            .covariance()
            .diagonal()
            .iter()
            .copied()
            .fold(f32::NEG_INFINITY, f32::max);
        if max_variance < STATIONARY_THRESHOLD {
            Hypothesis::Stationary(max_variance)
        } else {
            Hypothesis::Moving(max_variance)
        }
    }

    pub fn predict(&mut self) {
        let f = |p: BallPosition| p;
        if let Err(err) = self.position_kf.predict(f, self.prediction_noise) {
            error!("failed to predict ball position: {err:?}");
        }
    }

    pub fn measurement_update(&mut self, measurement: BallPosition) {
        let h = |p: BallPosition| p;
        if self.check_field_boundaries(measurement) == false {
            return;
        }
        if let Err(err) = self.position_kf.update(h, measurement, self.sensor_noise) {
            error!("failed to do measurement update: {err:?}");
        }

        // Putting timestamp update here for now
        self.timestamp = Instant::now();
    }

    fn check_field_boundaries(&self, p: BallPosition) -> bool {
        // Proper boundaries are -4.5 <= x <= 4.5 and -3.0 <= y <= 3.0
        p.x >= -5.0 && p.x <= 5.0 && p.y >= -3.5 && p.y <= 3.5
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
