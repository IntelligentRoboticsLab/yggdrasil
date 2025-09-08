use std::sync::Arc;

use rerun::{SerializedComponentBatch, external::arrow};

/// Creates a [`SerializedComponentBatch`] from an iterator of values.
pub trait SerializeComponentBatch {
    #[must_use]
    fn serialize_component_batch<I: IntoIterator<Item = Self>>(
        descriptor: &str,
        iter: I,
    ) -> SerializedComponentBatch;
}

impl SerializeComponentBatch for f32 {
    fn serialize_component_batch<I: IntoIterator<Item = Self>>(
        descriptor: &str,
        iter: I,
    ) -> SerializedComponentBatch {
        rerun::SerializedComponentBatch::new(
            Arc::new(arrow::array::Float32Array::from_iter_values(iter)),
            rerun::ComponentDescriptor::new(descriptor),
        )
    }
}

impl SerializeComponentBatch for u64 {
    fn serialize_component_batch<I: IntoIterator<Item = Self>>(
        descriptor: &str,
        iter: I,
    ) -> SerializedComponentBatch {
        rerun::SerializedComponentBatch::new(
            Arc::new(arrow::array::UInt64Array::from_iter_values(iter)),
            rerun::ComponentDescriptor::new(descriptor),
        )
    }
}
