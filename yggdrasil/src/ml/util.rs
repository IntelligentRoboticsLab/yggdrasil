//! Utility functions for machine learning.

/// Returns the index of the maximum element in a [`Vec`].
pub fn argmax(v: &[f32]) -> usize {
    v.iter()
        .enumerate()
        .max_by(|(_, v1), (_, v2)| v1.total_cmp(v2))
        .expect("argmax: empty vector")
        .0
}

/// Returns the softmax of [`Vec`].
pub fn softmax(v: &[f32]) -> Vec<f32> {
    let exps = v.iter().map(|f| f.exp()).collect::<Vec<_>>();

    let sum: f32 = exps.iter().sum();
    exps.iter().map(|x| x / sum).collect()
}
