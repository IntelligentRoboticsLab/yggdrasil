//! Trait implementations for data types that are used in a ML model as
//! in- and output.
//!
//! This involves the type system in defining and utilizing models.

pub mod input;
pub mod output;

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
    };
}

/// A data type that can serve as input and output of an [`MlModel`].
///
/// # Safety
///
/// OpenVINO internally stores data in tensors with some precision (aka data type),
/// but when this data is requested it's returned as a byte buffer.
/// To judge if casting to a type that implements [`Elem`] is safe,
/// [`Elem::is_compatible`] is called. The rest of the implementation
/// relies on the fact that this method functions correctly, or else
/// we wind up with undefined behavior.
pub unsafe trait Elem: Sized + Send + Sync + 'static {
    /// Returns `true` if `Self` is compatible with
    /// `precision`, i.e. instances of `precision` can be safely
    /// interpreted as instances of `Self`.
    fn is_compatible(precision: openvino::Precision) -> bool;

    /// Returns a view to the bytes of a slice of `Self`s.
    fn view_bytes_slice(slice: &[Self]) -> &[u8] {
        let ptr = slice.as_ptr() as *const u8;
        let len = slice.len() * std::mem::size_of::<Self>() / std::mem::size_of::<u8>();

        // Safety: the pointer is valid and the length is correct.
        unsafe { std::slice::from_raw_parts(ptr, len) }
    }
}

unsafe impl Elem for u8 {
    fn is_compatible(_: openvino::Precision) -> bool {
        // we can interpret any data type as bytes.
        true
    }
}

impl_elem!(unsafe { f32 => openvino::Precision::FP32 });
impl_elem!(unsafe { f64 => openvino::Precision::FP64 });
impl_elem!(unsafe { u32 => openvino::Precision::U32 });
impl_elem!(unsafe { i32 => openvino::Precision::I32 });
// NOTE: implement for more types if necessary
