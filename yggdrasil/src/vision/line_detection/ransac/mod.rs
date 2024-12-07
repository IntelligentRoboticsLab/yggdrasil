pub mod line;

pub trait Ransac: Sized {
    const MIN_SAMPLES: usize;

    type Model;
    type Data;

    fn next(&mut self) -> Option<(Self::Model, Vec<Self::Data>)>;

    fn residual(&self, model: &Self::Model, point: &Self::Data) -> f32;
}
