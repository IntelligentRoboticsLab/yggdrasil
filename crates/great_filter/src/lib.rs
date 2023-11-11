pub mod pose;

use nalgebra::{Cholesky, RealField, SMatrix, SVector};

type State<T, const D: usize> = SVector<T, D>;
type Cov<T, const D: usize> = SMatrix<T, D, D>;
type CrossCov<T, const D: usize, const D2: usize> = SMatrix<T, D, D2>;

#[derive(Debug, Clone)]
pub struct MultivariateGaussian<T: RealField + Copy, const DIM: usize> {
    pub mean: State<T, DIM>,
    pub covariance: Cov<T, DIM>,
}

impl<T: RealField + Copy, const DIM: usize> MultivariateGaussian<T, DIM> {
    pub fn with_x_0(x_0: State<T, DIM>) -> Self {
        // x_hat
        let mean = x_0.mean();

        // x - x_hat
        let centered = x_0 - State::repeat(mean);

        // Cov(x,x) = E[(x - x_hat)(x - x_hat)^T]
        let covariance =
            (centered * centered.transpose()) / nalgebra::convert::<f64, T>(DIM as f64);

        Self {
            mean: State::repeat(mean),
            covariance,
        }
    }
}

/// Common value for sigma points: 2 * STATE_DIM + 1
#[derive(Debug)]
pub struct UkfState<T: RealField + Copy, const STATE_DIM: usize, const SIGMA_POINTS: usize> {
    /// The current estimate
    pub gaussian: MultivariateGaussian<T, STATE_DIM>,

    /// Weights associated with each sigma point
    w_m: SVector<T, SIGMA_POINTS>,
    w_c: SVector<T, SIGMA_POINTS>,
}

impl<T: RealField + Copy, const STATE_DIM: usize, const SIGMA_POINTS: usize>
    UkfState<T, STATE_DIM, SIGMA_POINTS>
{
    /// Common parameter values:
    ///
    /// alpha = 0.001, beta = 2, kappa = 0
    pub fn new(alpha: T, beta: T, kappa: T, x_0: State<T, STATE_DIM>) -> Self {
        let gaussian = MultivariateGaussian::with_x_0(x_0);
        let (w_m, w_c) = Self::calculate_weights(alpha, beta, kappa);

        Self { gaussian, w_m, w_c }
    }

    fn calculate_sigma_points(&self) -> [SVector<T, STATE_DIM>; SIGMA_POINTS] {
        let mut sigma_points = [SVector::<T, STATE_DIM>::zeros(); SIGMA_POINTS];
        let MultivariateGaussian { mean, covariance } = self.gaussian;

        // nalgebra generics is evil / const generic arithmetic is nightly so we just do this for now
        // we need an uneven amount of sigma points so they are centered around the mean
        assert!(SIGMA_POINTS % 2 != 0);

        // get the lower triangular matrix from our cholesky decomposition
        let l = Cholesky::new(covariance)
            .expect("Failed to perform Cholesky decomposition")
            .l();

        // s_0 = mean
        sigma_points[0] = mean;

        // s_1, ..., s_n = mean + l (column i)
        for i in 0..STATE_DIM {
            sigma_points[i + 1] = mean + l.column(i);
        }

        // s_n+1, ..., s_2n = mean + l (column i)
        for i in 0..STATE_DIM {
            sigma_points[i + STATE_DIM + 1] = mean - l.column(i);
        }

        sigma_points
    }

    fn calculate_weights(
        alpha: T,
        beta: T,
        kappa: T,
    ) -> (SVector<T, SIGMA_POINTS>, SVector<T, SIGMA_POINTS>) {
        let n = nalgebra::convert(SIGMA_POINTS as f64);

        // alpha^2 * (n + kappa) - n
        let lambda = T::powi(alpha, 2) * (n + kappa) - n;

        // lambda / (n + lambda)
        let w_m_0 = lambda / (n + lambda);

        // w_m_0 + 1 - alpha^2 + beta
        let w_c_0 = w_m_0 + (nalgebra::convert::<f64, T>(1.0) - T::powi(alpha, 2) + beta);

        // 1 / (2 * (n + lambda))
        let w_m_i =
            nalgebra::convert::<f64, T>(1.0) / (nalgebra::convert::<f64, T>(2.0) * (n + lambda));
        let w_c_i = w_m_i;

        let mut w_m = SVector::<T, SIGMA_POINTS>::repeat(w_m_i);
        w_m[0] = w_m_0;
        let mut w_c = SVector::<T, SIGMA_POINTS>::repeat(w_c_i);
        w_c[0] = w_c_0;

        (w_m, w_c)
    }

    fn predict(
        &mut self,
        transition_function: impl Fn(State<T, STATE_DIM>) -> State<T, STATE_DIM>,
        transition_noise: &Cov<T, STATE_DIM>,
    ) {
        let sigma_points: Vec<State<T, STATE_DIM>> = self
            .calculate_sigma_points()
            .into_iter()
            // get the updated sigma point positions
            .map(transition_function)
            .collect();

        let sigma_points_mean: State<T, STATE_DIM> =
            sigma_points.iter().sum::<State<T, STATE_DIM>>()
                / nalgebra::convert::<f64, T>(sigma_points.len() as f64);

        // calculate new state mean and covariance
        let (mean, mut covariance) = sigma_points
            .iter()
            .zip(self.w_m.iter())
            .zip(self.w_c.iter())
            .fold(
                (State::<T, STATE_DIM>::zeros(), Cov::<T, STATE_DIM>::zeros()),
                |(mean, covariance), ((sigma_point, &w_m), &w_c)| {
                    let centered_sigma_point: State<T, STATE_DIM> = sigma_point - sigma_points_mean;

                    let new_mean = mean + sigma_point * w_m;

                    let new_covariance = covariance
                        + (centered_sigma_point * centered_sigma_point.transpose()) * w_c
                        + transition_noise;

                    (new_mean, new_covariance)
                },
            );

        // might not be needed?? b-human & hulks both do this correction step with their covariances
        if covariance != covariance.transpose() {
            println!("Bruh!!! Covariance is not symmetrical\n{covariance}");
            covariance = (covariance + covariance.transpose()) / nalgebra::convert::<f64, T>(2.0);
        }

        // set new state with prediction
        self.gaussian = MultivariateGaussian { mean, covariance };
    }

    // update/innovate
    fn update<const MEASUREMENT_DIM: usize>(
        &mut self,
        measurement_function: impl Fn(State<T, STATE_DIM>) -> State<T, MEASUREMENT_DIM>,
        measurement: State<T, MEASUREMENT_DIM>,
        measurement_noise: Cov<T, MEASUREMENT_DIM>,
    ) {
        let sigma_points = self.calculate_sigma_points();

        let predicted_measurements: Vec<State<T, MEASUREMENT_DIM>> = sigma_points
            .iter()
            .copied()
            // get sigma point their predicted measurements
            .map(measurement_function)
            .collect();

        // predicted mean of transformed points
        let predicted_mean = predicted_measurements.iter().zip(self.w_m.iter()).fold(
            State::<T, MEASUREMENT_DIM>::zeros(),
            |mean, (predicted_measurement, &w_m)| mean + predicted_measurement * w_m,
        );

        // predicted covariance of transformed points
        let mut predicted_covariance = predicted_measurements.iter().zip(self.w_c.iter()).fold(
            Cov::<T, MEASUREMENT_DIM>::zeros(),
            |covariance, (predicted_measurement, &w_c)| {
                let centered_measurement = predicted_measurement - predicted_mean;
                covariance
                    + (centered_measurement * centered_measurement.transpose()) * w_c
                    + measurement_noise
            },
        );

        // might not be needed?? b-human & hulks both do this correction step with their covariances
        if predicted_covariance != predicted_covariance.transpose() {
            println!("Bruh!!! Predicted covariance is not symmetrical\n{predicted_covariance}");
            predicted_covariance = (predicted_covariance + predicted_covariance.transpose())
                / nalgebra::convert::<f64, T>(2.0);
        }

        // cross covariance matrix between sigma points and predicted measurements
        let cross_covariance = predicted_measurements
            .iter()
            .zip(sigma_points.iter())
            .zip(self.w_c.iter())
            .fold(
                CrossCov::<T, STATE_DIM, MEASUREMENT_DIM>::zeros(),
                |covariance, ((predicted_measurement, sigma_point), &w_c)| {
                    let centered_measurement = predicted_measurement - predicted_mean;
                    let centered_sigma_point = sigma_point - sigma_points[0];

                    covariance + (centered_sigma_point * centered_measurement.transpose()) * w_c
                },
            );

        // K_k = C_xz - S_hat_k^-1
        let kalman_gain = cross_covariance
            * predicted_covariance
                .try_inverse()
                .expect("Failed to get inverse");

        // innovation = z_k - z_hat
        // x_hat_k|k = x_hat_k|k-1 + K_k (innovation)
        let mean = self.gaussian.mean + kalman_gain * (measurement - predicted_mean);
        // P_k|k = P_k|k-1 - K_k S_hat_k K_k^T
        let covariance =
            self.gaussian.covariance - kalman_gain * predicted_covariance * kalman_gain.transpose();

        self.gaussian = MultivariateGaussian { mean, covariance };
    }
}
