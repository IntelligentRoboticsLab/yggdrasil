use std::sync::Arc;

use rerun::{SerializedComponentBatch, external::arrow};

/// Creates a [`SerializedComponentBatch`] from an iterator of `f32` values.
#[must_use]
pub fn serialized_component_batch_f32<I: IntoIterator<Item = f32>>(
    descriptor: &str,
    iter: I,
) -> SerializedComponentBatch {
    rerun::SerializedComponentBatch::new(
        Arc::new(arrow::array::Float32Array::from_iter_values(iter)),
        rerun::ComponentDescriptor::new(descriptor),
    )
}
