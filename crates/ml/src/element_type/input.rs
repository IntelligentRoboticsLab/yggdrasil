use super::{Elem, MlArray};

pub trait InputContainer<E: Elem>: Sized {
    fn view_byte_slice(&self) -> &[u8];
}

impl<E: Elem> InputContainer<E> for Vec<E> {
    fn view_byte_slice(&self) -> &[u8] {
        Elem::view_bytes_slice(self)
    }
}

impl<E: Elem> InputContainer<E> for MlArray<E> {
    fn view_byte_slice(&self) -> &[u8] {
        Elem::view_bytes_slice(self.as_slice().expect("failed to get slice from MlArray"))
    }
}

pub trait ModelInput<E: Elem> {
    type Shape;

    fn view_byte_slices(&self) -> Vec<&[u8]>;
}

macro_rules! impl_model_input {
    ($(($P:ident, $p:ident)),*) => {
        impl<E: Elem, $($P: InputContainer<E>),*> ModelInput<E> for ($($P),*,) {
            type Shape = ($($P),*,);

            #[allow(unused_assignments)]
            fn view_byte_slices(&self) -> Vec<&[u8]> {
                let ($($p),*,) = self;
                vec![$($p.view_byte_slice(),)*]
            }
        }
    };
}

bevy::utils::all_tuples!(impl_model_input, 1, 15, I, i);
