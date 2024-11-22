use std::{fmt::Debug, marker::PhantomData};

use nalgebra::{Cholesky, SMatrix, SVector};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Covariance matrix is not definite-positive")]
    Cholesky,
    #[error("Matrix is not invertible")]
    Inversion,
}

pub type Result<T> = std::result::Result<T, Error>;

pub type StateVector<const D: usize> = SVector<f32, D>;
pub type CovarianceMatrix<const D: usize> = SMatrix<f32, D, D>;
pub type CrossCovarianceMatrix<const D1: usize, const D2: usize> = SMatrix<f32, D1, D2>;

pub type SigmaPoints1 = SigmaPoints<1, 3>;
pub type SigmaPoints2 = SigmaPoints<2, 5>;
pub type SigmaPoints3 = SigmaPoints<3, 7>;
pub type SigmaPoints4 = SigmaPoints<4, 9>;

/// Merwe scaled sigma points
///
/// N should be `2 * D + 1` where D is the dimension of your state vector
#[derive(Debug)]
pub struct SigmaPoints<const D_STATE: usize, const N_SIGMAS: usize> {
    pub alpha: f32,
    pub beta: f32,
    pub kappa: f32,
    /// weights for means and covariances
    pub w_m: SVector<f32, N_SIGMAS>,
    pub w_c: SVector<f32, N_SIGMAS>,
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
    ) -> (SVector<f32, N_SIGMAS>, SVector<f32, N_SIGMAS>) {
        let d = D_STATE as f32;

        let a_squared_k = alpha.powi(2) * kappa;

        let w = 1.0 / (2.0 * a_squared_k);
        let mut w_m = SVector::<f32, N_SIGMAS>::repeat(w);
        let mut w_c = SVector::<f32, N_SIGMAS>::repeat(w);

        w_m[0] = (a_squared_k - d) / a_squared_k;
        w_c[0] = w_m[0] + 1.0 - alpha.powi(2) + beta;

        (w_m, w_c)
    }

    /// Calculate the new sigma points from a state mean and covariance
    pub fn calculate(
        &self,
        mean: StateVector<D_STATE>,
        covariance: CovarianceMatrix<D_STATE>,
    ) -> Result<SMatrix<f32, N_SIGMAS, D_STATE>> {
        // get the lower triangular matrix from cholesky decomposition
        let cholesky_l = Cholesky::new(covariance).ok_or(Error::Cholesky)?.l();

        let mut sigma_points = SMatrix::<f32, N_SIGMAS, D_STATE>::zeros();

        // s_0 = mean
        sigma_points.set_row(0, &mean.transpose());

        for i in 0..D_STATE {
            let u = self.alpha * self.kappa.sqrt() * cholesky_l.column(i);
            // s_1, ..., s_n = mean + alpha * sqrt(kappa) * l.T_i
            sigma_points.set_row(i + 1, &(mean + u).transpose());
            // s_n+1, ..., s_2n = mean - alpha * sqrt(kappa) * l.T_i
            sigma_points.set_row(i + 1 + D_STATE, &(mean - u).transpose());
        }

        Ok(sigma_points)
    }
}

/// An Unscented Kalman Filter
///
/// Uses the formulation found [here](https://nbviewer.org/github/sbitzer/UKF-exposed/blob/master/UKF.ipynb)
pub struct UnscentedKalmanFilter<
    const D_STATE: usize,
    const N_SIGMAS: usize,
    State: UkfState<D_STATE>,
> {
    sigmas: SigmaPoints<D_STATE, N_SIGMAS>,
    state: State,
    covariance: CovarianceMatrix<D_STATE>,
    _measurement: PhantomData<State>,
}

impl<const D_STATE: usize, const N_SIGMAS: usize, State: UkfState<D_STATE>>
    UnscentedKalmanFilter<D_STATE, N_SIGMAS, State>
{
    /// Creates self from a state and covariance, with the default sigma points parameters
    #[must_use]
    pub fn new<C>(state_0: State, covariance_0: C) -> Self
    where
        C: Into<CovarianceMatrix<D_STATE>>,
    {
        Self::with_sigma_points(
            SigmaPoints::new(1.0, 0.0, D_STATE as f32 * 3.0 / 2.0),
            state_0,
            covariance_0,
        )
    }

    /// Creates self from a state, covariance, and a set of sigma points.
    ///
    /// If you don't know which parameters to use, you probably want to use [`UnscentedKalmanFilter::new`] instead
    #[must_use]
    pub fn with_sigma_points<C>(
        sigmas: SigmaPoints<D_STATE, N_SIGMAS>,
        state_0: State,
        covariance_0: C,
    ) -> Self
    where
        C: Into<CovarianceMatrix<D_STATE>>,
    {
        Self {
            sigmas,
            state: state_0,
            covariance: covariance_0.into(),
            _measurement: PhantomData,
        }
    }

    /// The current predicted filter state
    #[must_use]
    pub fn state(&self) -> State {
        self.state
    }

    /// The current covariance of the filter state
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
        F: Fn(State) -> State,
    {
        let state_vec: StateVector<D_STATE> = self.state.into();
        let sigma_points = self.sigmas.calculate(state_vec, self.covariance)?;

        // apply the motion model to each sigma point
        let transformed_sigma_points = sigma_points
            .row_iter()
            .map(|row| transition_function(row.transpose().into()));

        // calculate the new state mean and covariance
        let mean = UkfState::into_state_mean(
            self.sigmas
                .w_m
                .iter()
                .copied()
                .zip(transformed_sigma_points.clone()),
        );

        let covariance: CovarianceMatrix<D_STATE> = {
            // start with additive process noise
            let mut covariance = transition_noise;

            for (i, point) in transformed_sigma_points.enumerate() {
                let centered: StateVector<D_STATE> = point.residual(&mean).into();
                covariance += self.sigmas.w_c[i] * centered * centered.transpose();
            }

            covariance
        };

        self.state = mean;
        self.covariance = covariance;

        Ok(())
    }

    /// Updates the filter state with a measurement
    pub fn update<const D_MEASUREMENT: usize, Measurement, F>(
        &mut self,
        measurement_function: F,
        measurement: Measurement,
        measurement_noise: CovarianceMatrix<D_MEASUREMENT>,
    ) -> Result<()>
    where
        Measurement: UkfState<D_MEASUREMENT>,
        F: Fn(State) -> Measurement,
    {
        let state_vec = self.state.into();
        let sigma_points_matrix = self.sigmas.calculate(state_vec, self.covariance)?;

        let sigma_points = sigma_points_matrix
            .row_iter()
            .map(|row| row.transpose().into());

        // apply the measurement model to each sigma point
        let transformed_sigma_points = sigma_points.clone().map(&measurement_function);

        // calculate the new state mean and covariance
        let mean = UkfState::into_state_mean(
            self.sigmas
                .w_m
                .iter()
                .copied()
                .zip(transformed_sigma_points.clone()),
        );

        let covariance: CovarianceMatrix<D_MEASUREMENT> = {
            // start with additive measurement noise
            let mut covariance = measurement_noise;

            for (i, point) in transformed_sigma_points.clone().enumerate() {
                let centered: StateVector<D_MEASUREMENT> = point.residual(&mean).into();
                covariance += self.sigmas.w_c[i] * centered * centered.transpose();
            }

            covariance
        };

        let cross_covariance: CrossCovarianceMatrix<D_STATE, D_MEASUREMENT> = {
            let mut cross_covariance = CrossCovarianceMatrix::<D_STATE, D_MEASUREMENT>::zeros();

            for (i, (transformed_sigma_point, sigma_point)) in
                transformed_sigma_points.zip(sigma_points).enumerate()
            {
                // we need to get the residual the measurement
                let measurement_centered: StateVector<D_MEASUREMENT> =
                    transformed_sigma_point.residual(&mean).into();

                // and also our predicted current motion state
                let motion_centered: StateVector<D_STATE> =
                    sigma_point.residual(&self.state).into();

                cross_covariance +=
                    self.sigmas.w_c[i] * motion_centered * measurement_centered.transpose();
            }

            cross_covariance
        };

        let kalman_gain = cross_covariance * covariance.try_inverse().ok_or(Error::Inversion)?;
        let innovation: StateVector<D_MEASUREMENT> = measurement.residual(&mean).into();

        self.state = (state_vec + kalman_gain * innovation).into();
        self.covariance -= kalman_gain * covariance * kalman_gain.transpose();

        Ok(())
    }
}

/// Trait that describes how to transform state in the Unscented Kalman Filter
pub trait UkfState<const D: usize>
where
    Self: From<StateVector<D>> + Into<StateVector<D>> + Copy,
{
    /// Calculates the mean state from an iterator over weights and sigma points
    fn into_state_mean<T>(iter: T) -> Self
    where
        T: Iterator<Item = (f32, Self)>;

    /// Calculates the residual (difference) between self and the other value.
    #[must_use]
    fn residual(self, other: &Self) -> Self;
}
