//! Implementing Encoding and Decoding for types from [`nalgebra`]

use std::io::{Read, Write};

use nalgebra::{Dim, Matrix, Scalar, Storage, StorageMut};

use super::{Decode, Encode};

use crate::Result;

impl<T, R, C, S> Encode for Matrix<T, R, C, S>
where
    T: Scalar + Encode,
    R: Dim,
    C: Dim,
    S: Storage<T, R, C>,
{
    fn encode(&self, mut write: impl Write) -> Result<()> {
        for item in self.iter() {
            item.encode(&mut write)?;
        }
        Ok(())
    }

    fn encode_len(&self) -> usize {
        let mut total_encode_len = 0;

        for elem in self {
            total_encode_len += elem.encode_len();
        }

        total_encode_len
    }
}

impl<T, R, C, S> Decode for Matrix<T, R, C, S>
where
    T: Scalar + Decode,
    R: Dim,
    C: Dim,
    S: StorageMut<T, R, C> + Default,
{
    fn decode(mut read: impl Read) -> Result<Self>
    where
        Self: Sized,
    {
        let mut matrix = Matrix::<T, R, C, S>::default();
        let length = matrix.len();

        for index in 0..length {
            let item = matrix.index_mut(index);
            *item = T::decode(&mut read)?;
        }

        Ok(matrix)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::serialization::{Decode, Encode};
    use nalgebra::{Matrix2, Matrix3x6, Matrix4x2, Matrix6, Vector, Vector2, Vector3, Vector4};
    use std::fmt::Debug;

    fn test_matrix<T, R, C, S>(input: Matrix<T, R, C, S>) -> Result<()>
    where
        T: Encode + Decode + Debug + Scalar,
        R: Dim,
        C: Dim,
        S: StorageMut<T, R, C> + Default + Debug,
    {
        let mut encoded: Vec<u8> = Vec::new();
        input.encode(&mut encoded)?;
        let decoded = Matrix::<T, R, C, S>::decode(&mut encoded.as_slice())?;

        assert_eq!(input, decoded);
        assert_eq!(input.encode_len(), decoded.encode_len());
        assert_eq!(input.encode_len(), encoded.len());

        Ok(())
    }

    #[test]
    fn test_vectors() -> Result<()> {
        // Vector from nalgebra
        test_matrix(Vector2::from_element(u8::MAX))?;
        test_matrix(Vector3::from_element(u16::MAX))?;
        test_matrix(Vector4::from_element(f32::MAX))?;
        test_matrix(Vector::from([f64::MAX; 8]))?;

        Ok(())
    }

    #[test]
    fn test_matrices() -> Result<()> {
        // Matrix from nalgebra
        test_matrix(Matrix2::from_element(u8::MAX))?;
        test_matrix(Matrix4x2::from_element(u16::MAX))?;
        test_matrix(Matrix3x6::from_element(f32::MAX))?;
        test_matrix(Matrix6::from_element(f64::MAX))?;

        Ok(())
    }
}
