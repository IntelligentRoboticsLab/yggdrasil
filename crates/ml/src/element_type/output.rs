use super::{Elem, MlArray};
use openvino::Blob;

/// Output container of a ML model, where `E` is the element type that is contained.
pub trait OutputContainer<E: Elem>: Sized {
    /// Instantiate [`Self`] from a slice and the dimensions of the data.
    fn from_slice(slice: &[E], shape: &[usize]) -> Self;
}

impl<E: Elem + Clone> OutputContainer<E> for Vec<E> {
    fn from_slice(slice: &[E], _: &[usize]) -> Self {
        slice.to_vec()
    }
}

impl<E: Elem + Clone> OutputContainer<E> for MlArray<E> {
    fn from_slice(slice: &[E], shape: &[usize]) -> Self {
        // with the implementation of the backend this should never panic
        ndarray::Array::from_shape_vec(shape, slice.to_vec()).unwrap_or_else(|_| panic!(
            "Given shape does not match the number of elements in the slice (shape: {shape:?}, size: {})",
            slice.len()
        ))
    }
}

pub trait ModelOutput<E: Elem> {
    type Shape;
    fn from_blobs(blobs: &[Blob], shapes: &[Vec<usize>]) -> Self::Shape;
}

macro_rules! impl_model_output {
    ($($params:ident),*) => {
        impl<E: Elem, $($params: OutputContainer<E>),*> ModelOutput<E> for ($($params),*,) {
            type Shape = ($($params),*,);

            #[allow(unused_assignments)]
            fn from_blobs(blobs: &[Blob], shapes: &[Vec<usize>]) -> ($($params),*,) {
                let mut index = 0;
                (
                    $(
                        {
                            // SAFETY: the type of the blob is checked while creating the
                            // `ModelExecutor` instance.
                            let data = unsafe { blobs[index].buffer_as_type::<E>() }.unwrap();
                            let result = $params::from_slice(data, shapes[index].as_slice());
                            index += 1;
                            result
                        }
                    ),*,
                )
            }
        }
    };
}

bevy::utils::all_tuples!(impl_model_output, 1, 15, O);
