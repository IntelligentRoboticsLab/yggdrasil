//! Trait implementations for data types that are used in a ML model as
//! in- and output.
//!
//! This involves the type system in defining and utilizing models.

use bevy::utils::all_tuples;
use openvino::Tensor;

use crate::MlArray;

/// Implements [`DataType`] on a data type and maps it to an `OpenVINO` data type.
/// In other words, the data type can now be used as in- and output of a
/// ML model, granted that model uses the mapped `OpenVINO` data type internally.
///
/// Note that this is unsafe, see [`DataType`].
macro_rules! impl_datatype {
    (unsafe { $elem:ty => $element_type:path }) => {
        unsafe impl DataType for $elem {
            fn is_compatible(element_type: openvino::ElementType) -> bool {
                match element_type {
                    $element_type => true,
                    _ => false,
                }
            }

            fn element_type() -> openvino::ElementType {
                $element_type
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
    fn is_compatible(element_type: openvino::ElementType) -> bool;

    /// Returns the `OpenVINO` data type that corresponds to `Self`.
    fn element_type() -> openvino::ElementType;

    /// Returns a view to the bytes of a slice of `Self`s.
    fn as_blob(slice: &[Self]) -> &[u8] {
        let ptr = slice.as_ptr().cast::<u8>();
        let len = std::mem::size_of_val(slice) / std::mem::size_of::<u8>();

        // Safety: the pointer is valid and the length is correct.
        unsafe { std::slice::from_raw_parts(ptr, len) }
    }
}

unsafe impl DataType for u8 {
    fn is_compatible(_: openvino::ElementType) -> bool {
        // we can interpret any data type as bytes.
        true
    }

    fn element_type() -> openvino::ElementType {
        openvino::ElementType::U8
    }
}

impl_datatype!(unsafe { f32 => openvino::ElementType::F32 });
impl_datatype!(unsafe { f64 => openvino::ElementType::F64 });

impl_datatype!(unsafe { u16 => openvino::ElementType::U16 });
impl_datatype!(unsafe { u32 => openvino::ElementType::U32 });
impl_datatype!(unsafe { u64 => openvino::ElementType::U64 });

impl_datatype!(unsafe { i8 => openvino::ElementType::I8 });
impl_datatype!(unsafe { i16 => openvino::ElementType::I16 });
impl_datatype!(unsafe { i32 => openvino::ElementType::I32 });
impl_datatype!(unsafe { i64 => openvino::ElementType::I64 });
// NOTE: implement for more types if necessary

pub trait Parameters: Sized {
    /// Returns an iterator over the raw bytes blob for each model parameter.
    fn blobs(&self) -> impl Iterator<Item = &[u8]>;

    /// Returns the total amount of elements across all model parameters.
    fn num_elements(&self) -> usize;

    /// The data type of each model parameter.
    fn data_types() -> impl Iterator<Item = openvino::ElementType>;

    /// The byte size of the dtype of each model parameter.
    fn sizes_of() -> impl Iterator<Item = usize>;

    /// The amount of model parameters.
    #[must_use]
    fn len() -> usize {
        1
    }

    /// Creates a new instance of `Self` from the openvino input tensors.
    ///
    /// # Safety
    ///
    /// The tensor must be a valid representation of `Self`.
    unsafe fn from_tensors(iter: impl Iterator<Item = Tensor>) -> Self;
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

    fn data_types() -> impl Iterator<Item = openvino::ElementType> {
        std::iter::once(E::element_type())
    }

    fn sizes_of() -> impl Iterator<Item = usize> {
        std::iter::once(size_of::<E>())
    }

    unsafe fn from_tensors(mut iter: impl Iterator<Item = Tensor>) -> Self {
        let tensor = iter.next().unwrap();
        let slice: &[E] = tensor.get_data().unwrap();
        slice[0].clone()
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

    fn data_types() -> impl Iterator<Item = openvino::ElementType> {
        std::iter::once(E::element_type())
    }

    fn sizes_of() -> impl Iterator<Item = usize> {
        std::iter::once(size_of::<E>())
    }

    unsafe fn from_tensors<'a>(mut iter: impl Iterator<Item = Tensor>) -> Self {
        let tensor = iter.next().unwrap();
        let slice: &[E] = tensor.get_data().unwrap();
        slice.to_vec()
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

    fn data_types() -> impl Iterator<Item = openvino::ElementType> {
        std::iter::once(E::element_type())
    }

    fn sizes_of() -> impl Iterator<Item = usize> {
        std::iter::once(size_of::<E>())
    }

    unsafe fn from_tensors<'a>(mut iter: impl Iterator<Item = Tensor>) -> Self {
        let tensor = iter.next().unwrap();
        let slice: &[E] = tensor.get_data().unwrap();

        let shape = tensor.get_shape().unwrap();
        let dims = shape
            .get_dimensions()
            .iter()
            .map(|&dim| dim as usize)
            .collect::<Vec<_>>();

        MlArray::from_shape_vec(dims, slice.to_vec()).unwrap()
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

            fn data_types() -> impl Iterator<Item = openvino::ElementType> {
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


            unsafe fn from_tensors<'a>(mut iter: impl Iterator<Item = Tensor>) -> Self {
                (
                    $(
                        $T::from_tensors(iter.by_ref())
                    ,)*
                )
            }
        }
    };
}

all_tuples!(impl_parameters, 1, 8, T);
