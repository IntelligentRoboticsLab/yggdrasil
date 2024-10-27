use nalgebra::Vector2;
use rand::RngCore;
use sample_consensus::{Consensus, Estimator, Model};

pub struct Line {
    /// Normal to the line itself
    normal: Vector2<f32>,
    /// Distance to the origin
    d: f32,
}

impl Model<Vector2<f32>> for Line {
    fn residual(&self, point: &Vector2<f32>) -> f64 {
        (self.normal.dot(point) + self.d).abs() as f64
    }
}

struct LineEstimator;

impl Estimator<Vector2<f32>> for LineEstimator {
    type Model = Line;
    type ModelIter = std::iter::Once<Line>;
    const MIN_SAMPLES: usize = 2;

    fn estimate<I>(&self, mut data: I) -> Self::ModelIter
    where
        I: Iterator<Item = Vector2<f32>> + Clone,
    {
        let a = data.next().unwrap();
        let b = data.next().unwrap();

        let normal = Vector2::new(a.y - b.y, b.x - a.x).normalize();
        let d = -normal.dot(&b);
        std::iter::once(Line { normal, d })
    }
}

pub fn fit<R, I>(data: I, rng: R) -> Option<(Line, Vec<usize>)>
where
    R: RngCore,
    I: Iterator<Item = Vector2<f32>> + Clone,
{
    let mut arrsac = arrsac::Arrsac::new(0.5, rng);

    arrsac.model_inliers(&LineEstimator, data)
}
