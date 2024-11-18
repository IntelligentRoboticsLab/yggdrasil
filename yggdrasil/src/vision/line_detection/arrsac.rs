use nalgebra::{Point2, Vector2};
use rand::RngCore;
use sample_consensus::{Consensus, Estimator, Model};

use super::line::Line2;

impl Model<Point2<f32>> for Line2 {
    fn residual(&self, point: &Point2<f32>) -> f64 {
        f64::from((self.normal.dot(&point.coords) + self.d).abs())
    }
}

struct LineEstimator;

impl Estimator<Point2<f32>> for LineEstimator {
    type Model = Line2;
    type ModelIter = std::iter::Once<Line2>;
    const MIN_SAMPLES: usize = 2;

    fn estimate<I>(&self, mut data: I) -> Self::ModelIter
    where
        I: Iterator<Item = Point2<f32>> + Clone,
    {
        let a = data.next().unwrap();
        let b = data.next().unwrap();

        let normal = Vector2::new(a.y - b.y, b.x - a.x).normalize();
        let d = -normal.dot(&b.coords);
        std::iter::once(Line2 { normal, d })
    }
}

pub struct Arrsac<R: RngCore> {
    inner: arrsac::Arrsac<R>,
}

impl<R: RngCore> Arrsac<R> {
    pub fn new(threshold: f64, rng: R) -> Self {
        Self {
            inner: arrsac::Arrsac::new(threshold, rng),
        }
    }

    pub fn fit<I>(&mut self, data: I) -> Option<(Line2, Vec<usize>)>
    where
        I: Iterator<Item = Point2<f32>> + Clone,
    {
        self.inner.model_inliers(&LineEstimator, data)
    }
}
