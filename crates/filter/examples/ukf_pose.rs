use std::ops::Mul;

use filter::{StateVector, UkfState, UnscentedKalmanFilter};

use plotters::prelude::*;
use rand::Rng;

use nalgebra::{self as na, Complex, ComplexField, SVector, UnitComplex, Vector3};

const UPDATE_INTERVAL: usize = 5;

#[derive(Debug, Clone, Copy)]
struct Pose2 {
    inner: na::Isometry3<f32>,
}

impl Pose2 {
    fn new(x: f32, y: f32, theta: f32) -> Self {
        Self {
            inner: na::Isometry3::from_parts(
                na::Translation3::from(na::Vector3::new(x, y, 0.0)),
                na::UnitQuaternion::from_axis_angle(&Vector3::z_axis(), theta),
            ),
        }
    }
}

impl Mul for Pose2 {
    type Output = Self;

    /// Applies a motion to self
    fn mul(self, rhs: Self) -> Self::Output {
        Self {
            inner: rhs.inner * self.inner,
        }
    }
}

impl UkfState<3> for Pose2 {
    fn into_state_mean<T>(iter: T) -> Self
    where
        T: Iterator<Item = (f32, Self)>,
    {
        let mut mean_translation = SVector::zeros();
        let mut mean_angle = Complex::new(0.0, 0.0);

        for (weight, pose) in iter {
            let translation = pose.inner.translation.vector;
            let rotation = pose.inner.rotation.angle();

            mean_translation += weight * translation;
            mean_angle += weight * Complex::cis(rotation);
        }

        mean_translation.xy().push(mean_angle.argument()).into()
    }

    fn residual(self, other: &Self) -> Self {
        let self_state = StateVector::<3>::from(self);
        let other_state = StateVector::<3>::from(*other);

        (self_state.xy() - other_state.xy())
            .push((UnitComplex::new(self_state.z) / UnitComplex::new(other_state.z)).angle())
            .into()
    }
}

impl From<Pose2> for StateVector<3> {
    fn from(pose: Pose2) -> Self {
        let translation = pose.inner.translation.vector;
        let rotation = pose.inner.rotation;
        translation.xy().push(rotation.angle())
    }
}

impl From<StateVector<3>> for Pose2 {
    fn from(state: StateVector<3>) -> Self {
        Self {
            inner: na::Isometry3::from_parts(
                na::Translation3::from(state.xy().push(0.0)),
                na::UnitQuaternion::from_axis_angle(
                    &na::Unit::new_normalize(na::Vector3::z()),
                    state.z,
                ),
            ),
        }
    }
}

fn main() {
    let pose = Pose2::new(1.0, 2.0, 0.0);
    let cov = na::SMatrix::<f32, 3, 3>::identity() * 0.05;

    let mut ukf = UnscentedKalmanFilter::<3, 7, Pose2>::new(pose, cov);

    // generate measurements
    let mut rng = rand::thread_rng();
    let mut x_true = vec![];
    let mut x_noisy = vec![];
    let mut x_unscented = vec![];

    for i in 0..150 {
        let prev = x_true
            .last()
            .cloned()
            .unwrap_or_else(|| Pose2::new(1.0, 2.0, 0.0));

        let offset = Pose2::new(
            rng.gen_range(-0.001..0.001),
            rng.gen_range(-0.01..0.01),
            rng.gen_range(-0.01..0.01),
        ) * Pose2::new(-0.05, 0.1, -0.02);

        // true measurement
        let measurement = offset * prev;

        x_true.push(measurement.clone());

        let noisy_offset = Pose2::new(
            rng.gen_range(-1.0..1.0) * cov[(0, 0)],
            rng.gen_range(-1.0..1.0) * cov[(1, 1)],
            rng.gen_range(-1.0..1.0) * cov[(2, 2)],
        ) * offset;

        let noisy_prev = x_noisy
            .last()
            .cloned()
            .unwrap_or_else(|| Pose2::new(1.0, 2.0, 0.0));

        // noisy, dead reckoning measurement
        let noisy = noisy_offset * noisy_prev;

        x_noisy.push(noisy);

        ukf.predict(|p| noisy_offset * p, filter::CovarianceMatrix::identity())
            .unwrap();

        // Every nth step, updates the filter with a measurement
        //
        // Uses a very low covariance as we are very sure about our measurements
        if i % UPDATE_INTERVAL == 0 {
            ukf.update(
                |p| p,
                measurement,
                filter::CovarianceMatrix::identity() * 0.01,
            )
            .unwrap();
        }

        x_unscented.push(ukf.state());
    }

    // plot the results
    let root_area = BitMapBackend::new("./ukf_pose.png", (600, 400)).into_drawing_area();
    root_area.fill(&WHITE).unwrap();

    let mut ctx = ChartBuilder::on(&root_area)
        .set_label_area_size(LabelAreaPosition::Left, 40)
        .set_label_area_size(LabelAreaPosition::Bottom, 40)
        .margin_top(20)
        .margin_right(20)
        .build_cartesian_2d(0.0..10.0f32, 0.0..10.0f32)
        .unwrap();

    ctx.configure_mesh().draw().unwrap();

    ctx.draw_series(x_true.iter().map(|point| {
        let translation = point.inner.translation.vector;
        Circle::new((translation.x, translation.y), 2, RED.filled())
    }))
    .unwrap()
    .label("Ground truth")
    .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], RED));

    ctx.draw_series(x_noisy.iter().map(|point| {
        let translation = point.inner.translation.vector;
        Circle::new((translation.x, translation.y), 2, BLUE.filled())
    }))
    .unwrap()
    .label("Dead reckoning")
    .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], BLUE));

    ctx.draw_series(x_unscented.iter().map(|point| {
        let translation = point.inner.translation.vector;
        Circle::new((translation.x, translation.y), 2, GREEN.filled())
    }))
    .unwrap()
    .label("Ukf values")
    .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], GREEN));

    ctx.configure_series_labels()
        .border_style(BLACK)
        .background_style(WHITE.mix(0.8))
        .position(SeriesLabelPosition::UpperLeft)
        .draw()
        .unwrap();
}
