//! Implementing Encoding and Decoding for types from [`nalgebra`]

use std::io::{Read, Write};

use nalgebra::{
    allocator::Allocator, DefaultAllocator, Dim, RawStorageMut, Scalar, Storage, Vector,
};

use super::{Decode, Encode, VarInt};

use crate::Result;

impl<T, D, S> Encode for Vector<T, D, S>
where
    T: Scalar + Encode,
    D: Dim,
    S: Storage<T, D>,
{
    fn encode(&self, mut write: impl Write) -> Result<()> {
        VarInt::from(self.len()).encode(&mut write)?;

        for item in self.iter() {
            item.encode(&mut write)?;
        }
        Ok(())
    }

    fn encode_len(&self) -> usize {
        let mut total_encode_len = VarInt::from(self.len()).encode_len();

        for elem in self {
            total_encode_len += elem.encode_len();
        }

        total_encode_len
    }
}

impl<T, D, S> Decode for Vector<T, D, S>
where
    T: Scalar + Decode,
    D: Dim,
    S: RawStorageMut<T, D> + Default,
    DefaultAllocator: Allocator<D>,
{
    fn decode(mut read: impl Read) -> Result<Self>
    where
        Self: Sized,
    {
        let length = VarInt::decode(&mut read)?.into();

        let mut vec = Vector::<T, D, S>::default();

        for index in 0..length {
            let item = vec.index_mut(index);
            *item = T::decode(&mut read)?;
        }

        Ok(vec)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::serialization::codec::tests::test_generic;
    use nalgebra::{Vector, Vector2, Vector3, Vector4};

    #[test]
    fn test_vector() -> Result<()> {
        // Vector from nalgebra
        test_generic(Vector2::from_element(u8::MAX))?;
        test_generic(Vector3::from_element(u16::MAX))?;
        test_generic(Vector4::from_element(f32::MAX))?;
        test_generic(Vector::from([f64::MAX; 8]))?;

        Ok(())
    }
}
