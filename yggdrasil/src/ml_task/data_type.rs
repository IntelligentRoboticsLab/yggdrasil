/// Conveniency type representing an n-dimensional array.  
pub type MlArray<E> = ndarray::Array<E, ndarray::Dim<ndarray::IxDynImpl>>;

/// An element type that can serve as an
/// input, granted it implements [`InputElem`],
/// and output type of a ML model.
/// ## Safety
/// OpenVINO internally stores data in tensor with some precision/data type,
/// but when this data is requested it's returned as a byte buffer.
/// To judge if casting to a type that implements [`Elem`] is safe
/// [`Elem::is_compatible`] is called. The rest of the implementation
/// relies on the fact that this method functions correctly, or else
/// we end up with undefined behavior.
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

unsafe impl Elem for f32 {
    fn is_compatible(precision: openvino::Precision) -> bool {
        match precision {
            openvino::Precision::FP32 => true,
            _ => false,
        }
    }
}

/// Input element type of a ML model.
pub trait InputElem: Elem {
    /// Returns a view to the bytes of a slice of `Self`s.
    fn view_slice_bytes(slice: &[Self]) -> &[u8];
}

impl InputElem for u8 {
    fn view_slice_bytes(slice: &[Self]) -> &[u8] {
        slice
    }
}

impl InputElem for f32 {
    fn view_slice_bytes(slice: &[Self]) -> &[u8] {
        let ptr = slice.as_ptr() as *const u8;
        let len = slice.len() * std::mem::size_of::<f32>() / std::mem::size_of::<u8>();

        // this is a safe conversion
        unsafe { std::slice::from_raw_parts(ptr, len) }
    }
}

/// Output of a ML model, where `E` is the element type.
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
        ndarray::Array::from_shape_vec(shape, slice.to_vec()).expect(&format!(
            "Given shape does not match the number of elements in the slice (shape: {shape:?}, size: {})",
            slice.len()
        ))
    }
}
