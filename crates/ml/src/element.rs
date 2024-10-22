//! Trait implementations for data types that are used in a ML model as
//! in- and output.
//!
//! This involves the type system in defining and utilizing models.

use bevy::utils::all_tuples;
use openvino::Blob;

use crate::MlArray;

/// Implements [`DataType`] on a data type and maps it to an `OpenVINO` data type.
/// In other words, the data type can now be used as in- and output of a
/// ML model, granted that model uses the mapped `OpenVINO` data type internally.
///
/// Note that this is unsafe, see [`DataType`].
macro_rules! impl_datatype {
    (unsafe { $elem:ty => $precision:path }) => {
        unsafe impl DataType for $elem {
            fn is_compatible(precision: openvino::Precision) -> bool {
                match precision {
                    $precision => true,
                    _ => false,
                }
            }

            fn precision() -> openvino::Precision {
                $precision
            }
        }
    };
}

/// A data type that can serve as input and output of an [`MlModel`](`super::MlModel`).
///
/// # Safety
///
/// `OpenVINO` internally stores data in tensors with some precision (aka data type),
/// but when this data is requested it's returned as a byte buffer.
/// To judge if casting to a type that implements [`DataType`] is safe,
/// [`DataType::is_compatible`] is called.
/// The rest of the implementation relies on the fact that this method functions
/// correctly, or else we wind up with undefined behavior.
pub unsafe trait DataType: Clone + Sized + Send + Sync + 'static {
    /// Returns `true` if `Self` is compatible with
    /// `precision`, i.e. instances of `precision` can be safely
    /// interpreted as instances of `Self`.
    #[allow(dead_code)]
    fn is_compatible(precision: openvino::Precision) -> bool;

    /// Returns the `OpenVINO` data type that corresponds to `Self`.
    fn precision() -> openvino::Precision;

    /// Returns a view to the bytes of a slice of `Self`s.
    fn as_blob(slice: &[Self]) -> &[u8] {
        let ptr = slice.as_ptr().cast::<u8>();
        let len = std::mem::size_of_val(slice) / std::mem::size_of::<u8>();

        // Safety: the pointer is valid and the length is correct.
        unsafe { std::slice::from_raw_parts(ptr, len) }
    }
}

unsafe impl DataType for u8 {
    fn is_compatible(_: openvino::Precision) -> bool {
        // we can interpret any data type as bytes.
        true
    }

    fn precision() -> openvino::Precision {
        openvino::Precision::U8
    }
}

impl_datatype!(unsafe { f32 => openvino::Precision::FP32 });
impl_datatype!(unsafe { f64 => openvino::Precision::FP64 });

impl_datatype!(unsafe { u16 => openvino::Precision::U16 });
impl_datatype!(unsafe { u32 => openvino::Precision::U32 });
impl_datatype!(unsafe { u64 => openvino::Precision::U64 });

impl_datatype!(unsafe { i8 => openvino::Precision::I8 });
impl_datatype!(unsafe { i16 => openvino::Precision::I16 });
impl_datatype!(unsafe { i32 => openvino::Precision::I32 });
impl_datatype!(unsafe { i64 => openvino::Precision::I64 });
// NOTE: implement for more types if necessary

pub trait Parameters: Sized {
    /// Returns an iterator over the raw bytes blob for each model parameter.
    fn blobs(&self) -> impl Iterator<Item = &[u8]>;

    /// Returns the total amount of elements across all model parameters.
    fn num_elements(&self) -> usize;

    /// The data type of each model parameter.
    fn data_types() -> impl Iterator<Item = openvino::Precision>;

    /// The size of each model parameter.
    fn sizes_of() -> impl Iterator<Item = usize>;

    /// The amount of model parameters.
    #[must_use]
    fn len() -> usize {
        1
    }

    /// Creates a new instance of `Self` from a byte blob.
    ///
    /// # Safety
    ///
    /// The blob must be a valid representation of `Self`.
    unsafe fn from_dims_and_blobs<'a>(iter: impl Iterator<Item = (&'a [usize], Blob)>) -> Self;
}

impl<E> Parameters for E
where
    E: DataType,
{
    fn blobs(&self) -> impl Iterator<Item = &[u8]> {
        std::iter::once(DataType::as_blob(std::slice::from_ref(self)))
    }

    fn num_elements(&self) -> usize {
        1
    }

    fn data_types() -> impl Iterator<Item = openvino::Precision> {
        std::iter::once(E::precision())
    }

    fn sizes_of() -> impl Iterator<Item = usize> {
        std::iter::once(size_of::<E>())
    }

    unsafe fn from_dims_and_blobs<'a>(mut iter: impl Iterator<Item = (&'a [usize], Blob)>) -> Self {
        let (_, blob) = iter.next().unwrap();

        let values = blob
            .buffer_as_type::<E>()
            .expect("Failed to cast blob to output type");

        values[0].clone()
    }
}

impl<E> Parameters for Vec<E>
where
    E: DataType,
{
    fn blobs(&self) -> impl Iterator<Item = &[u8]> {
        std::iter::once(DataType::as_blob(self.as_slice()))
    }

    fn num_elements(&self) -> usize {
        self.len()
    }

    fn data_types() -> impl Iterator<Item = openvino::Precision> {
        std::iter::once(E::precision())
    }

    fn sizes_of() -> impl Iterator<Item = usize> {
        std::iter::once(size_of::<E>())
    }

    unsafe fn from_dims_and_blobs<'a>(mut iter: impl Iterator<Item = (&'a [usize], Blob)>) -> Self {
        let (_, blob) = iter.next().unwrap();

        let values = blob
            .buffer_as_type::<E>()
            .expect("Failed to cast blob to output type");

        values.to_vec()
    }
}

impl<E> Parameters for MlArray<E>
where
    E: DataType,
{
    fn blobs(&self) -> impl Iterator<Item = &[u8]> {
        std::iter::once(DataType::as_blob(self.as_slice().unwrap()))
    }

    fn num_elements(&self) -> usize {
        self.len()
    }

    fn data_types() -> impl Iterator<Item = openvino::Precision> {
        std::iter::once(E::precision())
    }

    fn sizes_of() -> impl Iterator<Item = usize> {
        std::iter::once(size_of::<E>())
    }

    unsafe fn from_dims_and_blobs<'a>(mut iter: impl Iterator<Item = (&'a [usize], Blob)>) -> Self {
        let (dims, blob) = iter.next().unwrap();

        let values = blob
            .buffer_as_type::<E>()
            .expect("Failed to cast blob to output type");

        MlArray::from_shape_vec(dims, values.to_vec()).unwrap()
    }
}

macro_rules! impl_parameters {
    ($($T:ident),*) =>
    {
        #[allow(non_snake_case)]

        impl<$($T: Parameters),*> Parameters for ($($T,)*) {
            fn blobs(&self) -> impl Iterator<Item = &[u8]> {
                let ($($T,)*) = self;
                std::iter::empty()
                    $(
                        .chain($T.blobs())
                    )*
            }

            fn num_elements(&self) -> usize {
                let ($($T,)*) = self;
                0 $(+ $T.num_elements())*
            }

            fn data_types() -> impl Iterator<Item = openvino::Precision> {
                std::iter::empty()
                    $(
                        .chain($T::data_types())
                    )*
            }

            fn sizes_of() -> impl Iterator<Item = usize> {
                std::iter::empty()
                    $(
                        .chain($T::sizes_of())
                    )*
            }

            fn len() -> usize {
                0 $(+ $T::len())*
            }


            unsafe fn from_dims_and_blobs<'a>(mut iter: impl Iterator<Item = (&'a [usize], Blob)>) -> Self {
                (
                    $(
                        $T::from_dims_and_blobs(iter.by_ref())
                    ,)*
                )
            }
        }
    };
}

all_tuples!(impl_parameters, 1, 8, T);
