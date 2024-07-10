//! Trait implementations for data types that are used in a ML model as
//! in- and output. This involves the type system in defining and utilizing models.

/// Conveniency type representing an n-dimensional array.
pub type MlArray<E> = ndarray::Array<E, ndarray::Dim<ndarray::IxDynImpl>>;

/// Implements [`Elem`] on a data type and maps it to an OpenVINO data type.
/// In other words, the data type can now be used as in- and output of a
/// ML model, granted that model uses the mapped OpenVINO data type internally.
///
/// Note that this is unsafe, see [`Elem`].
macro_rules! impl_elem {
    (unsafe { $elem:ty => $precision:path }) => {
        unsafe impl Elem for $elem {
            fn is_compatible(precision: openvino::Precision) -> bool {
                match precision {
                    $precision => true,
                    _ => false,
                }
            }
        }

        impl InputElem for $elem {
            fn view_slice_bytes(slice: &[Self]) -> &[u8] {
                let ptr = slice.as_ptr() as *const u8;
                let len = slice.len() * std::mem::size_of::<$elem>() / std::mem::size_of::<u8>();

                // this is a safe conversion
                unsafe { std::slice::from_raw_parts(ptr, len) }
            }
        }
    };
}

/// A data type that can serve as
/// input, granted it implements [`InputElem`],
/// and output of a ML model.
///
/// ## Safety
/// OpenVINO internally stores data in tensors with some precision (aka data type),
/// but when this data is requested it's returned as a byte buffer.
/// To judge if casting to a type that implements [`Elem`] is safe,
/// [`Elem::is_compatible`] is called. The rest of the implementation
/// relies on the fact that this method functions correctly, or else
/// we wind up with undefined behavior.
pub unsafe trait Elem: Sized {
    /// Returns `true` if `Self` is compatible with
    /// `precision`, i.e. instances of `precision` can be safely
    /// interpreted as instances of `Self`.
    fn is_compatible(precision: openvino::Precision) -> bool;
}

unsafe impl Elem for u8 {
    fn is_compatible(_: openvino::Precision) -> bool {
        // we can interpret any data type as bytes.
        true
    }
}

/// Input element type of a ML model. The reason this
/// trait is separate from [`Elem`] is that a user should technically
/// be allowed to implement just [`Elem`] (and not [`InputElem`])
/// for an output type and use another type as input.
pub trait InputElem: Elem {
    /// Returns a view to the bytes of a slice of `Self`s.
    fn view_slice_bytes(slice: &[Self]) -> &[u8];
}

impl InputElem for u8 {
    fn view_slice_bytes(slice: &[Self]) -> &[u8] {
        slice
    }
}

impl_elem!(unsafe { f32 => openvino::Precision::FP32 });
impl_elem!(unsafe { f64 => openvino::Precision::FP64 });
impl_elem!(unsafe { u32 => openvino::Precision::U32 });
impl_elem!(unsafe { i32 => openvino::Precision::I32 });
// NOTE: implement for more types if necessary

/// Input container of a ML model, where `E` is the element type that is contained.
pub trait Input<E: Elem>: Sized {
    fn view_slice_bytes(&self) -> &[u8];
}

impl<E: InputElem> Input<E> for Vec<E> {
    fn view_slice_bytes(&self) -> &[u8] {
        InputElem::view_slice_bytes(self)
    }
}

impl<E: InputElem> Input<E> for MlArray<E> {
    fn view_slice_bytes(&self) -> &[u8] {
        InputElem::view_slice_bytes(self.as_slice().unwrap())
    }
}

/// Output container of a ML model, where `E` is the element type that is contained.
pub trait Output<E: Elem>: Sized {
    /// Instantiate `Self` from a slice and the dimensions of the data.
    fn from_slice(slice: &[E], shape: &[usize]) -> Self;
}

impl<E: Elem + Clone> Output<E> for Vec<E> {
    fn from_slice(slice: &[E], _: &[usize]) -> Self {
        slice.to_vec()
    }
}

impl<E: Elem + Clone> Output<E> for MlArray<E> {
    fn from_slice(slice: &[E], shape: &[usize]) -> Self {
        // with the implementation of the backend this should never panic
        ndarray::Array::from_shape_vec(shape, slice.to_vec()).unwrap_or_else(|_| panic!(
            "Given shape does not match the number of elements in the slice (shape: {shape:?}, size: {})",
            slice.len()
        ))
    }
}

impl<E: Elem + Clone> Output<E> for (MlArray<E>, MlArray<E>, MlArray<E>) {
    fn from_slice(slice: &[E], shape: &[usize]) -> Self {
        (
            MlArray::from_shape_vec(shape, slice.to_vec()).unwrap_or_else(|_| panic!(
                "Given shape does not match the number of elements in the slice (shape: {shape:?}, size: {})",
                slice.len()
            )),
            MlArray::from_shape_vec(shape, slice.to_vec()).unwrap_or_else(|_| panic!(
                "Given shape does not match the number of elements in the slice (shape: {shape:?}, size: {})",
                slice.len()
            )),
            MlArray::from_shape_vec(shape, slice.to_vec()).unwrap_or_else(|_| panic!(
                "Given shape does not match the number of elements in the slice (shape: {shape:?}, size: {})",
                slice.len()
            )),
        )
    }
}
