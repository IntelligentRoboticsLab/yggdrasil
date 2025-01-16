pub mod line;

/// Trait for random sample consensus (RANSAC) algorithms.
pub trait Ransac: Sized {
    /// Amount of samples required to fit a model.
    const MIN_SAMPLES: usize;

    /// The model that is fitted to the data.
    type Model;
    /// The data that is used to fit the model.
    type Data;

    fn next(&mut self) -> Option<(Self::Model, Vec<Self::Data>)>;
}
