//! # Filter
//!
//! This crate provides a set of filtering algorithms and utilities to help you filter your noisy ahh data.

use std::{fmt::Debug, marker::PhantomData};

use nalgebra::{Cholesky, SMatrix, SVector};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Covariance matrix is not positive-definite")]
    Cholesky,
    #[error("Matrix is not invertible")]
    Inversion,
}

pub type Result<T> = std::result::Result<T, Error>;

/// The weight of a sigma point
pub type Weight = f32;

pub type StateVector<const D: usize> = SVector<f32, D>;
pub type WeightVector<const N: usize> = SVector<Weight, N>;
pub type StateMatrix<const D: usize, const N: usize> = SMatrix<f32, D, N>;
pub type CovarianceMatrix<const D: usize> = SMatrix<f32, D, D>;
pub type CrossCovarianceMatrix<const D1: usize, const D2: usize> = SMatrix<f32, D1, D2>;

pub type SigmaPoints1 = SigmaPoints<1, 3>;
pub type SigmaPoints2 = SigmaPoints<2, 5>;
pub type SigmaPoints3 = SigmaPoints<3, 7>;
pub type SigmaPoints4 = SigmaPoints<4, 9>;

/// Merwe scaled sigma points
///
/// N should be `2 * D + 1` where D is the dimension of your state vector
#[derive(Debug, Clone, Copy)]
pub struct SigmaPoints<const D_STATE: usize, const N_SIGMAS: usize> {
    pub alpha: f32,
    pub beta: f32,
    pub kappa: f32,
    /// weights for means and covariances
    pub w_m: SVector<Weight, N_SIGMAS>,
    pub w_c: SVector<Weight, N_SIGMAS>,
}

impl<const D_STATE: usize, const N_SIGMAS: usize> SigmaPoints<D_STATE, N_SIGMAS> {
    // TODO: if const generic arithmetic stabilizes we can remove the N_SIGMAS generic parameter.
    const ASSERT_CONST_PARAMS: () = assert!(2 * D_STATE + 1 == N_SIGMAS);

    /// A typical recommendation is alpha = 1, beta = 0, and kappa ≈ 3D / 2.
    ///
    /// If the true distribution is Gaussian, beta = 2 is optimal.
    #[must_use]
    pub fn new(alpha: f32, beta: f32, kappa: f32) -> Self {
        let () = Self::ASSERT_CONST_PARAMS;

        let (w_m, w_c) = Self::calculate_weights(alpha, beta, kappa);

        Self {
            alpha,
            beta,
            kappa,
            w_m,
            w_c,
        }
    }

    fn calculate_weights(
        alpha: f32,
        beta: f32,
        kappa: f32,
    ) -> (WeightVector<N_SIGMAS>, WeightVector<N_SIGMAS>) {
        let d = D_STATE as f32;

        let a_squared_k = alpha.powi(2) * kappa;

        let w = 1.0 / (2.0 * a_squared_k);
        let mut w_m = SVector::<Weight, N_SIGMAS>::repeat(w);
        let mut w_c = SVector::<Weight, N_SIGMAS>::repeat(w);

        w_m[0] = (a_squared_k - d) / a_squared_k;
        w_c[0] = w_m[0] + 1.0 - alpha.powi(2) + beta;

        (w_m, w_c)
    }

    /// Calculate the new sigma points from a state mean and covariance
    pub fn calculate(
        &self,
        mean: StateVector<D_STATE>,
        covariance: CovarianceMatrix<D_STATE>,
    ) -> Result<StateMatrix<D_STATE, N_SIGMAS>> {
        // get the lower triangular matrix from cholesky decomposition
        let cholesky_l = Cholesky::new(covariance).ok_or(Error::Cholesky)?.l();

        let mut sigma_points = SMatrix::<Weight, D_STATE, N_SIGMAS>::zeros();

        // s_0 = mean
        sigma_points.set_column(0, &mean);

        for i in 0..D_STATE {
            let u = self.alpha * self.kappa.sqrt() * cholesky_l.column(i);
            // s_1, ..., s_n = mean + alpha * sqrt(kappa) * l.T_i
            sigma_points.set_column(i + 1, &(mean + u));
            // s_n+1, ..., s_2n = mean - alpha * sqrt(kappa) * l.T_i
            sigma_points.set_column(i + 1 + D_STATE, &(mean - u));
        }

        Ok(sigma_points)
    }
}

/// An Unscented Kalman Filter
///
/// Uses the formulation found [here](https://nbviewer.org/github/sbitzer/UKF-exposed/blob/master/UKF.ipynb)
#[derive(Debug, Clone, Copy)]
pub struct UnscentedKalmanFilter<const D_STATE: usize, const N_SIGMAS: usize, S>
where
    S: StateTransform<D_STATE>,
{
    sigmas: SigmaPoints<D_STATE, N_SIGMAS>,
    pub state: StateVector<D_STATE>,
    pub covariance: CovarianceMatrix<D_STATE>,

    _state_transform: PhantomData<S>,
}

impl<const D_STATE: usize, const N_SIGMAS: usize, S: StateTransform<D_STATE>>
    UnscentedKalmanFilter<D_STATE, N_SIGMAS, S>
{
    /// Creates self from a state and covariance, with the default sigma points parameters
    #[must_use]
    pub fn new(state: S, covariance: CovarianceMatrix<D_STATE>) -> Self {
        Self::with_sigma_points(
            SigmaPoints::new(1.0, 0.0, D_STATE as f32 * 3.0 / 2.0),
            state,
            covariance,
        )
    }

    /// Creates self from a state, covariance, and a set of sigma points.
    ///
    /// If you don't know which parameters to use, you probably want to use [`UnscentedKalmanFilter::new`] instead
    #[must_use]
    pub fn with_sigma_points(
        sigmas: SigmaPoints<D_STATE, N_SIGMAS>,
        state: S,
        covariance: CovarianceMatrix<D_STATE>,
    ) -> Self {
        Self {
            sigmas,
            state: state.into(),
            covariance,
            _state_transform: PhantomData,
        }
    }

    /// The predicted filter state
    #[must_use]
    pub fn state(&self) -> S {
        self.state.into()
    }

    /// The current filter state covariance
    #[must_use]
    pub fn covariance(&self) -> CovarianceMatrix<D_STATE> {
        self.covariance
    }

    /// Predict the next filter state based on the motion transition model and process noise.
    pub fn predict<F>(
        &mut self,
        transition_function: F,
        transition_noise: CovarianceMatrix<D_STATE>,
    ) -> Result<()>
    where
        F: Fn(S) -> S,
    {
        let sigma_points = self.sigmas.calculate(self.state, self.covariance)?;

        // apply the motion model to each sigma point
        let transformed_sigma_points =
            Self::transform_sigma_points(sigma_points, |s| transition_function(s.into()).into());

        let (mean, covariance) = unscented_transform::<D_STATE, N_SIGMAS, S>(
            transformed_sigma_points,
            transition_noise,
            self.sigmas.w_m,
            self.sigmas.w_c,
        );

        self.state = mean;
        self.covariance = covariance;

        Ok(())
    }

    fn transform_sigma_points<const D_FROM: usize, const D_TO: usize>(
        sigma_points: StateMatrix<D_FROM, N_SIGMAS>,
        transform: impl Fn(StateVector<D_FROM>) -> StateVector<D_TO>,
    ) -> StateMatrix<D_TO, N_SIGMAS> {
        let mut transformed_sigma_points = SMatrix::<f32, D_TO, N_SIGMAS>::zeros();
        for (i, sigma_point) in sigma_points.column_iter().enumerate() {
            transformed_sigma_points.set_column(i, &transform(sigma_point.into_owned()));
        }
        transformed_sigma_points
    }

    /// Updates the filter state with a measurement
    pub fn update<const D_MEASUREMENT: usize, M, F>(
        &mut self,
        measurement_function: F,
        measurement: M,
        measurement_noise: CovarianceMatrix<D_MEASUREMENT>,
    ) -> Result<()>
    where
        M: StateTransform<D_MEASUREMENT>,
        F: Fn(S) -> M,
    {
        let measurement = measurement.into();

        let sigma_points = self.sigmas.calculate(self.state, self.covariance)?;

        // apply the measurement model to each sigma point
        let transformed_sigma_points =
            Self::transform_sigma_points(sigma_points, |s| measurement_function(s.into()).into());

        let (mean, covariance) = unscented_transform::<D_MEASUREMENT, N_SIGMAS, M>(
            transformed_sigma_points,
            measurement_noise,
            self.sigmas.w_m,
            self.sigmas.w_c,
        );

        let cross_covariance: CrossCovarianceMatrix<D_STATE, D_MEASUREMENT> = {
            let mut cross_covariance = CrossCovarianceMatrix::<D_STATE, D_MEASUREMENT>::zeros();

            for (i, (transformed_sigma_point, sigma_point)) in transformed_sigma_points
                .column_iter()
                .zip(sigma_points.column_iter())
                .enumerate()
            {
                // we need to get the residual the measurement
                let measurement_centered = M::residual(transformed_sigma_point.into_owned(), mean);

                // and also our predicted current motion state
                let motion_centered = S::residual(sigma_point.into_owned(), self.state);

                cross_covariance +=
                    self.sigmas.w_c[i] * motion_centered * measurement_centered.transpose();
            }

            cross_covariance
        };

        let kalman_gain = cross_covariance * covariance.try_inverse().ok_or(Error::Inversion)?;
        let innovation = M::residual(measurement, mean);

        self.state += kalman_gain * innovation;
        self.covariance -= kalman_gain * covariance * kalman_gain.transpose();

        Ok(())
    }
}

/// Performs the Unscented Transform on a set of sigma points
fn unscented_transform<const D_STATE: usize, const N_SIGMAS: usize, S: StateTransform<D_STATE>>(
    transformed_sigma_points: StateMatrix<D_STATE, N_SIGMAS>,
    mut covariance: CovarianceMatrix<D_STATE>,
    w_m: SVector<Weight, N_SIGMAS>,
    w_c: SVector<Weight, N_SIGMAS>,
) -> (StateVector<D_STATE>, CovarianceMatrix<D_STATE>) {
    let mean = S::into_state_mean(w_m, transformed_sigma_points);

    for (&weight, sigma_point) in w_c.iter().zip(transformed_sigma_points.column_iter()) {
        let residual: StateVector<D_STATE> = S::residual(sigma_point.into_owned(), mean);
        covariance += weight * residual * residual.transpose();
    }

    (mean, covariance)
}

/// Trait that describes how to transform state in the Unscented Kalman Filter
pub trait StateTransform<const D: usize>
where
    Self: From<StateVector<D>> + Into<StateVector<D>> + Sized,
{
    /// Calculates the mean state from an iterator over weights and sigma points
    #[must_use]
    fn into_state_mean<const N: usize>(
        weights: SVector<Weight, N>,
        states: SMatrix<f32, D, N>,
    ) -> StateVector<D> {
        states * weights
    }

    /// Calculates the residual (difference) between a measurement and the filter prediction.
    #[must_use]
    fn residual(measurement: StateVector<D>, prediction: StateVector<D>) -> StateVector<D> {
        measurement - prediction
    }
}

/// Calculates the Mahalanobis distance between a point and a distribution.
///
/// The Mahalanobis distance measures how many standard deviations away a point is from
/// the mean of a distribution, taking into account the covariance of the distribution.
pub fn mahalanobis_distance<const D: usize>(
    point: StateVector<D>,
    mean: StateVector<D>,
    covariance: CovarianceMatrix<D>,
) -> Result<f32> {
    // compute how far away the point is from the mean, this is the "residual"
    let diff = point - mean;

    // sqrt((x - mu)^T Sigma^-1 (x - mu))
    let cov_inv = covariance.try_inverse().ok_or(Error::Inversion)?;
    let distance_squared = diff.transpose() * cov_inv * diff;

    Ok(distance_squared.x.sqrt())
}

/// Extension trait to add Mahalanobis distance calculation to CovarianceMatrix
pub trait MahalanobisDistance<const D: usize> {
    /// Calculates the Mahalanobis distance between a point and a distribution with this covariance matrix.
    fn mahalanobis_distance(&self, point: StateVector<D>, mean: StateVector<D>) -> Result<f32>;
}

impl<const D: usize> MahalanobisDistance<D> for CovarianceMatrix<D> {
    fn mahalanobis_distance(&self, point: StateVector<D>, mean: StateVector<D>) -> Result<f32> {
        mahalanobis_distance(point, mean, *self)
    }
}
