use filter::{StateVec, UkfState, UnscentedKalmanFilter};

use plotters::prelude::*;
use rand::Rng;

use nalgebra::{self as na, Complex, ComplexField, SVector, UnitComplex, Vector3};

const UPDATE_INTERVAL: usize = 5;

#[derive(Debug, Clone)]
struct Pose {
    isom: na::Isometry3<f32>,
}

impl Pose {
    fn new(x: f32, y: f32, theta: f32) -> Self {
        Self {
            isom: na::Isometry3::from_parts(
                na::Translation3::from(na::Vector3::new(x, y, 0.0)),
                na::UnitQuaternion::from_axis_angle(&Vector3::z_axis(), theta),
            ),
        }
    }

    fn apply_motion(&self, offset: &Self) -> Self {
        Self {
            isom: offset.isom * self.isom,
        }
    }
}

impl UkfState<3> for Pose {
    fn into_weighted_mean<T>(iter: T) -> StateVec<3>
    where
        T: Iterator<Item = (f32, Self)>,
    {
        let mut mean_translation = SVector::zeros();
        let mut mean_angle = Complex::new(0.0, 0.0);

        for (weight, pose) in iter {
            let translation = pose.isom.translation.vector;
            let rotation = pose.isom.rotation.angle();

            mean_translation += weight * translation;
            mean_angle += weight * Complex::cis(rotation);
        }

        mean_translation.xy().push(mean_angle.argument())
    }

    fn center(self, mean: &StateVec<3>) -> StateVec<3> {
        let translation = self.isom.translation.vector;
        let rotation = self.isom.rotation.angle();

        (translation.xy() - mean.xy())
            .push((UnitComplex::new(rotation) / UnitComplex::new(mean.z)).angle())
    }
}

impl From<Pose> for StateVec<3> {
    fn from(pose: Pose) -> Self {
        let translation = pose.isom.translation.vector;
        let rotation = pose.isom.rotation;
        translation.xy().push(rotation.angle())
    }
}

impl From<StateVec<3>> for Pose {
    fn from(state: StateVec<3>) -> Self {
        Self {
            isom: na::Isometry3::from_parts(
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
    let pose = Pose::new(1.0, 2.0, 0.0);
    let cov = na::SMatrix::<f32, 3, 3>::identity() * 0.05;

    let mut ukf = UnscentedKalmanFilter::<3, 7, Pose>::new(pose, cov);

    // generate 100 noisy measurements
    let mut rng = rand::thread_rng();
    let mut x_true = vec![];
    let mut x_noisy = vec![];
    let mut x_unscented = vec![];

    for i in 0..150 {
        let prev = x_true
            .last()
            .cloned()
            .unwrap_or_else(|| Pose::new(1.0, 2.0, 0.0));

        let offset = Pose::new(-0.05, 0.1, -0.02).apply_motion(&Pose::new(
            rng.gen_range(-0.001..0.001),
            rng.gen_range(-0.01..0.01),
            rng.gen_range(-0.01..0.01),
        ));

        let measurement = prev.apply_motion(&offset);

        x_true.push(measurement.clone());

        let noisy_offset = offset.apply_motion(&Pose::new(
            rng.gen_range(-1.0..1.0) * cov[(0, 0)],
            rng.gen_range(-1.0..1.0) * cov[(1, 1)],
            rng.gen_range(-1.0..1.0) * cov[(2, 2)],
        ));

        let noisy_prev = x_noisy
            .last()
            .cloned()
            .unwrap_or_else(|| Pose::new(1.0, 2.0, 0.0));

        let noisy = noisy_prev.apply_motion(&noisy_offset);

        x_noisy.push(noisy);

        ukf.predict(
            |p| p.apply_motion(&noisy_offset),
            filter::CovMat::identity(),
        )
        .unwrap();

        // Every nth step, update the filter with a measurement
        //
        // Uses a very low covariance as we are very sure about our measurements
        if i % UPDATE_INTERVAL == 0 {
            ukf.update(|p| p, measurement, filter::CovMat::identity() * 0.001)
                .unwrap();
        }

        x_unscented.push(ukf.state().clone());
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
        let translation = point.isom.translation.vector;
        Circle::new((translation.x, translation.y), 2, RED.filled())
    }))
    .unwrap()
    .label("Ground truth")
    .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], RED));

    ctx.draw_series(x_noisy.iter().map(|point| {
        let translation = point.isom.translation.vector;
        Circle::new((translation.x, translation.y), 2, BLUE.filled())
    }))
    .unwrap()
    .label("Dead reckoning")
    .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], BLUE));

    ctx.draw_series(x_unscented.iter().map(|point| {
        let translation = point.isom.translation.vector;
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
