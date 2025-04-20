use std::fmt::Debug;

use bevy::utils::all_tuples;
use ort::{
    session::Input,
    tensor::{IntoTensorElementType, PrimitiveTensorElementType},
    value::{Tensor, Value},
};

use crate::MlArray;

trait DataType: PrimitiveTensorElementType + IntoTensorElementType {}
impl DataType for f32 {}
impl DataType for u8 {}

pub trait Parameters: Sized {
    fn num_elements(&self) -> usize;

    fn iter_data<'a>(
        &self,
        input_descriptions: impl Iterator<Item = &'a Input>,
    ) -> impl Iterator<Item = Value>;

    fn from_tensors<'a>(iter: impl Iterator<Item = &'a Value>) -> Self;
}

impl<T: DataType + Copy + Debug + 'static> Parameters for T {
    fn num_elements(&self) -> usize {
        1
    }

    fn iter_data<'a>(
        &self,
        mut input_descriptions: impl Iterator<Item = &'a Input>,
    ) -> impl Iterator<Item = Value> {
        let input_description = input_descriptions.next().unwrap();
        let dimension = input_description.input_type.tensor_dimensions().unwrap();
        let tensor = Tensor::from_array((dimension.as_slice(), vec![*self])).unwrap();
        std::iter::once(tensor.into())
    }

    fn from_tensors<'a>(mut iter: impl Iterator<Item = &'a Value>) -> Self {
        let value = iter.next().unwrap();

        if !value.is_tensor() {
            panic!("Input is not a tensor");
        }

        let (_dim, tensor) = value.try_extract_raw_tensor::<T>().unwrap();
        tensor[0]
    }
}

impl<T: DataType + Copy + Debug + 'static> Parameters for Vec<T> {
    fn num_elements(&self) -> usize {
        self.len()
    }

    fn iter_data<'a>(
        &self,
        mut input_descriptions: impl Iterator<Item = &'a Input>,
    ) -> impl Iterator<Item = Value> {
        let input_description = input_descriptions.next().unwrap();
        let dimension = input_description.input_type.tensor_dimensions().unwrap();
        let tensor = Tensor::from_array((dimension.as_slice(), self.as_slice())).unwrap();
        std::iter::once(tensor.into())
    }

    fn from_tensors<'a>(mut iter: impl Iterator<Item = &'a Value>) -> Self {
        // TODO(Rick): This is not really DRY with the earlier impl
        let value = iter.next().unwrap();

        if !value.is_tensor() {
            panic!("Input is not a tensor");
        }

        let (_dims, tensor) = value.try_extract_raw_tensor::<T>().unwrap();
        tensor.to_vec()
    }
}

impl<T: DataType + Copy + Debug + 'static> Parameters for MlArray<T> {
    fn num_elements(&self) -> usize {
        self.len()
    }

    fn iter_data<'a>(
        &self,
        mut input_descriptions: impl Iterator<Item = &'a Input>,
    ) -> impl Iterator<Item = Value> {
        let data: Vec<T> = self.iter().map(|item| *item).collect();

        let input_description = input_descriptions.next().unwrap();
        let dimension = input_description.input_type.tensor_dimensions().unwrap();
        let tensor = Tensor::from_array((dimension.as_slice(), data)).unwrap();
        std::iter::once(tensor.into())
    }

    fn from_tensors<'a>(mut iter: impl Iterator<Item = &'a Value>) -> Self {
        // TODO(Rick): This is not really DRY with the earlier impl
        let value = iter.next().unwrap();

        if !value.is_tensor() {
            panic!("Input is not a tensor");
        }

        let (dims, tensor) = value.try_extract_raw_tensor::<T>().unwrap();
        let dims = dims.iter().map(|&dim| dim as usize).collect::<Vec<_>>();

        MlArray::from_shape_vec(dims, tensor.to_vec()).unwrap()
    }
}

macro_rules! impl_parameters {
    ($($T:ident),*) =>
    {
        #[allow(non_snake_case)]

        impl<$($T: Parameters),*> Parameters for ($($T,)*) {
            fn num_elements(&self) -> usize {
                let ($($T,)*) = self;
                0 $(+ $T.num_elements())*
            }

            fn iter_data<'a>(&self, input_descriptions: impl Iterator<Item = &'a Input>) -> impl Iterator<Item = Value> {
                let ($($T,)*) = self;
                let mut input_iter = input_descriptions.into_iter();
                std::iter::empty()
                    $(
                        .chain($T.iter_data(std::iter::once(input_iter.next().expect("not enough input descriptions"))))
                    )*
            }

            fn from_tensors<'a>(mut iter: impl Iterator<Item = &'a Value>) -> Self {
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
