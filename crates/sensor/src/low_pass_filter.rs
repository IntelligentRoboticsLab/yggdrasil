use nalgebra::SVector;
use std::f32::consts::PI;

/// First-order Butterworth low-pass filter
#[derive(Copy, Clone, Debug)]
pub struct ButterworthLpf<const N: usize> {
    alpha: f32,
    beta: f32,
    x: SVector<f32, N>,
    y: SVector<f32, N>,
}

impl<const N: usize> ButterworthLpf<N> {
    #[must_use]
    pub fn new(omega: f32) -> Self {
        let alpha = omega / (1. + omega);
        let beta = (1. - omega) / (1. + omega);

        Self {
            alpha,
            beta,
            x: [0.; N].into(),
            y: [0.; N].into(),
        }
    }

    #[must_use]
    pub fn with_cutoff_freq(freq: f32, dt: f32) -> Self {
        Self::new((PI * freq * dt).tan())
    }

    pub fn update(&mut self, x: SVector<f32, N>) -> SVector<f32, N> {
        let y = self.alpha * (x + self.x) + self.beta * self.y;
        self.x = x;
        self.y = y;
        y
    }

    #[must_use]
    pub fn state(&self) -> SVector<f32, N> {
        self.y
    }
}

/// Exponential low-pass filter.
#[derive(Copy, Clone, Debug)]
pub struct ExponentialLpf<const N: usize> {
    alpha: f32,
    y: SVector<f32, N>,
}

impl<const N: usize> ExponentialLpf<N> {
    #[must_use]
    pub fn new(alpha: f32) -> Self {
        Self {
            alpha,
            y: [0.; N].into(),
        }
    }

    pub fn update(&mut self, x: SVector<f32, N>) -> SVector<f32, N> {
        self.y = (1.0 - self.alpha) * self.y + self.alpha * x;
        self.y
    }

    #[must_use]
    pub fn state(&self) -> SVector<f32, N> {
        self.y
    }
}
