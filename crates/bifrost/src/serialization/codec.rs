//! Implementing Encoding and Decoding for primitive types
//! (u8, u16, u32, u64, i8, i16, i32, i64, f32, f64),
//! arrays, strings, vectors and the `SPLStandardMessage` struct.

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::io::{Read, Write};

use crate::{Error, Result};

/// The `Encode` trait allows objects to be encoded to raw bytes.
/// See [`Decode`] for decoding objects from raw bytes.
///
/// # Deriving
///
/// This trait can be implemented automatically for structs and enums by using
/// the [`Encode`][macro] derive macro. All components of the type must
/// implement `Encode`. Components are encoded in the order they appear in the
/// type definition.
///
/// The derive macro encodes the variant with a leading [`usize`] to specify which
/// variant for encodig/decoding.
///
/// ```no_run
/// use bifrost::{serialization::Encode};
///
/// #[derive(Encode)]
/// struct Foo {
///     foo: i32,
///     bar: u32,
///     baz: [f32; 3],
/// }
///
/// #[derive(Encode)]
/// enum Bar {
///     Foo, // variant id = 0
///     Bar, // variant id = 1
///     Baz  // variant id = 2
/// }
///
/// let value = Foo {
///     foo: 32,
///     bar: 2_u32,
///     baz: [1.5, 3.14, 2.718],
/// };
///
/// let mut buf = vec![];
/// value.encode(&mut buf).unwrap();
/// ```
///
/// [macro]: bifrost_derive::Encode

pub trait Encode {
    /// # Arguments
    ///
    /// * `write` - A [writer](std::io::Write). For example, a buffer or a file.
    /// # Returns
    ///
    /// * `Result<()>` - A Result type that returns an empty tuple on success or an error on failure.
    fn encode(&self, write: impl Write) -> Result<()>;

    /// # Returns
    ///
    /// * `usize` - The length of the encoded data.
    fn encode_len(&self) -> usize;
}

/// The `Decode` trait allows objects to be decoded from raw bytes.
/// See [`Encode`] for encoding objects into raw bytes.
///
/// # Deriving
///
/// This trait can be implemented automatically for structs and enums by using
/// the [`Decode`][macro] derive macro. All components of the type must
/// implement `Decode`. Components are encoded in the order they appear in the
/// type definition.
///
/// The derive macro decodes enums by first reading a [`usize`] to determine the variant.
///
///
/// ```
/// use bifrost::{Result, serialization::Decode};
/// use std::io::Read;
///
/// #[derive(Decode)]
/// struct Foo {
///     bar: u32,
/// }
///
/// #[derive(Decode)]
/// enum Bar {
///     Foo, // variant id = 0
///     Bar, // variant id = 1
///     Baz  // variant id = 2
/// }
///
/// // some vector with data
/// let mut buf = vec![1_u8, 5_u8, 5_u8, 3_u8];
///
/// let value = Foo::decode(&mut buf.as_slice()).unwrap();
/// ```
///
/// [macro]: bifrost_derive::Decode
pub trait Decode {
    /// # Arguments
    /// * `read` - A [reader](std::io::Read). For example, a buffer or a file.
    ///
    /// # Returns
    /// * `Result<Self>` - A Result type that returns the decoded data on success or an error on failure.
    fn decode(read: impl Read) -> Result<Self>
    where
        Self: Sized;
}

impl Encode for bool {
    fn encode(&self, mut write: impl Write) -> Result<()> {
        write.write_u8(u8::from(*self))?;
        Ok(())
    }

    fn encode_len(&self) -> usize {
        1
    }
}

impl Decode for bool {
    fn decode(mut read: impl Read) -> Result<Self>
    where
        Self: Sized,
    {
        Ok(read.read_u8()? == u8::from(true))
    }
}

impl Encode for u8 {
    fn encode(&self, mut write: impl Write) -> Result<()> {
        write.write_u8(*self)?;
        Ok(())
    }

    fn encode_len(&self) -> usize {
        1
    }
}

impl Decode for u8 {
    fn decode(mut read: impl Read) -> Result<Self>
    where
        Self: Sized,
    {
        Ok(read.read_u8()?)
    }
}

impl Encode for u16 {
    fn encode(&self, mut write: impl Write) -> Result<()> {
        write.write_u16::<LittleEndian>(*self)?;
        Ok(())
    }

    fn encode_len(&self) -> usize {
        2
    }
}

impl Decode for u16 {
    fn decode(mut read: impl Read) -> Result<Self>
    where
        Self: Sized,
    {
        Ok(read.read_u16::<LittleEndian>()?)
    }
}

impl Encode for u32 {
    fn encode(&self, mut write: impl Write) -> Result<()> {
        write.write_u32::<LittleEndian>(*self)?;
        Ok(())
    }

    fn encode_len(&self) -> usize {
        4
    }
}

impl Decode for u32 {
    fn decode(mut read: impl Read) -> Result<Self>
    where
        Self: Sized,
    {
        Ok(read.read_u32::<LittleEndian>()?)
    }
}

impl Encode for u64 {
    fn encode(&self, mut write: impl Write) -> Result<()> {
        write.write_u64::<LittleEndian>(*self)?;
        Ok(())
    }

    fn encode_len(&self) -> usize {
        8
    }
}

impl Decode for u64 {
    fn decode(mut read: impl Read) -> Result<Self>
    where
        Self: Sized,
    {
        Ok(read.read_u64::<LittleEndian>()?)
    }
}

impl Encode for f32 {
    fn encode(&self, mut write: impl Write) -> Result<()> {
        write.write_f32::<LittleEndian>(*self)?;
        Ok(())
    }

    fn encode_len(&self) -> usize {
        4
    }
}

impl Decode for f32 {
    fn decode(mut read: impl Read) -> Result<Self>
    where
        Self: Sized,
    {
        Ok(read.read_f32::<LittleEndian>()?)
    }
}

impl Encode for f64 {
    fn encode(&self, mut write: impl Write) -> Result<()> {
        write.write_f64::<LittleEndian>(*self)?;
        Ok(())
    }

    fn encode_len(&self) -> usize {
        8
    }
}

impl Decode for f64 {
    fn decode(mut read: impl Read) -> Result<Self>
    where
        Self: Sized,
    {
        Ok(read.read_f64::<LittleEndian>()?)
    }
}

impl Encode for i8 {
    fn encode(&self, mut write: impl Write) -> Result<()> {
        write.write_i8(*self)?;
        Ok(())
    }

    fn encode_len(&self) -> usize {
        1
    }
}

impl Decode for i8 {
    fn decode(mut read: impl Read) -> Result<Self>
    where
        Self: Sized,
    {
        Ok(read.read_i8()?)
    }
}

impl Encode for i16 {
    fn encode(&self, mut write: impl Write) -> Result<()> {
        write.write_i16::<LittleEndian>(*self)?;
        Ok(())
    }

    fn encode_len(&self) -> usize {
        2
    }
}

impl Decode for i16 {
    fn decode(mut read: impl Read) -> Result<Self>
    where
        Self: Sized,
    {
        Ok(read.read_i16::<LittleEndian>()?)
    }
}

impl Encode for i32 {
    fn encode(&self, mut write: impl Write) -> Result<()> {
        write.write_i32::<LittleEndian>(*self)?;
        Ok(())
    }

    fn encode_len(&self) -> usize {
        4
    }
}

impl Decode for i32 {
    fn decode(mut read: impl Read) -> Result<Self>
    where
        Self: Sized,
    {
        Ok(read.read_i32::<LittleEndian>()?)
    }
}

impl Encode for i64 {
    fn encode(&self, mut write: impl Write) -> Result<()> {
        write.write_i64::<LittleEndian>(*self)?;
        Ok(())
    }

    fn encode_len(&self) -> usize {
        8
    }
}

impl Decode for i64 {
    fn decode(mut read: impl Read) -> Result<Self>
    where
        Self: Sized,
    {
        Ok(read.read_i64::<LittleEndian>()?)
    }
}

impl Encode for String {
    fn encode(&self, mut write: impl Write) -> Result<()> {
        VarInt::from(self.len()).encode(&mut write)?;
        write.write_all(self.as_bytes())?;

        Ok(())
    }

    fn encode_len(&self) -> usize {
        self.len() + VarInt::from(self.len()).encode_len()
    }
}

impl Decode for String {
    fn decode(mut read: impl Read) -> Result<Self>
    where
        Self: Sized,
    {
        let length = VarInt::decode(&mut read)?.into();
        let mut buf = vec![0; length];
        read.read_exact(&mut buf)?;

        Ok(String::from_utf8(buf)?)
    }
}

impl<T, const N: usize> Encode for [T; N]
where
    T: Encode,
{
    fn encode(&self, mut write: impl Write) -> Result<()> {
        for item in self {
            item.encode(&mut write)?;
        }
        Ok(())
    }

    fn encode_len(&self) -> usize {
        match self.len() {
            0 => 0,
            length => self[0].encode_len() * length,
        }
    }
}

impl<T, const N: usize> Decode for [T; N]
where
    T: Decode,
    T: Copy,
{
    fn decode(mut read: impl Read) -> Result<Self>
    where
        Self: Sized,
    {
        let mut arr = [T::decode(&mut read)?; N];
        for item in arr.iter_mut().skip(1) {
            *item = T::decode(&mut read)?;
        }
        Ok(arr)
    }
}

impl<T> Encode for Vec<T>
where
    T: Encode + num::traits::int::PrimInt,
{
    fn encode(&self, mut write: impl Write) -> Result<()> {
        VarInt::from(self.len()).encode(&mut write)?;

        for item in self {
            item.encode(&mut write)?;
        }
        Ok(())
    }

    fn encode_len(&self) -> usize {
        match self.len() {
            0 => VarInt::from(0).encode_len(),
            length => self[0].encode_len() * length + VarInt::from(self.len()).encode_len(),
        }
    }
}

impl<T> Decode for Vec<T>
where
    T: Decode + num::traits::int::PrimInt,
{
    fn decode(mut read: impl Read) -> Result<Self>
    where
        Self: Sized,
    {
        let length = VarInt::decode(&mut read)?.into();

        let mut vec = Vec::with_capacity(length);

        for _ in 0..length {
            vec.push(T::decode(&mut read)?);
        }

        Ok(vec)
    }
}

/// Calculates the amount of bytes needed to encode the zigzag encoded integer.
/// If the continuation bit of the byte in question is 1, the loop continues
/// and adds 1 to the amount of bytes needed to encode the integer.
fn required_encoded_size_unsigned(mut v: u64) -> usize {
    if v == 0 {
        return 1;
    }

    let mut size = 0;
    while v > 0 {
        size += 1;
        v >>= 7;
    }
    size
}

/// Calculates the amount of bytes needed to encode an i64 by first
/// converting it into its zigzag encoding.
fn required_encoded_space_signed(v: i64) -> usize {
    required_encoded_size_unsigned(zigzag_encode(v))
}

/// Uses the zigzag encoding in order to encode negative integers.
/// This is an alternative encoding to two's complement, proposed by Google.
/// <https://protobuf.dev/programming-guides/encoding/>
fn zigzag_encode(from: i64) -> u64 {
    ((from << 1) ^ (from >> 63)) as u64
}

fn zigzag_decode(from: u64) -> i64 {
    ((from >> 1) ^ (-((from & 1) as i64)) as u64) as i64
}

/// A wrapper around an integer.
///
/// Implements the [`Encode`] and [`Decode`] trait using [protobuf](https://github.com/protocolbuffers/protobuf) variable-length integers.
/// This allows small integer values to be encoded using less bytes then the byte size of the integer.
#[derive(Debug, PartialEq, Clone, Copy)]
pub struct VarInt<T> {
    value: T,
}

impl<T> VarInt<T> {
    /// Calculates the max size of bytes an unsigned integer can take up.
    /// Takes the size of the type in bytes and divides it by 7.
    pub const MAX_BYTES_UNSIGNED: usize =
        ((std::mem::size_of::<T>() * 8 + 7 - 1) as f64 / 7_f64) as usize;

    /// Calculates the max size of bytes an signed integer can take up.
    /// Takes the size of the type in bytes and divides it by 7.
    pub const MAX_BYTES_SIGNED: usize =
        ((std::mem::size_of::<T>() * 8 + 7 - 1 + 1) as f64 / 7_f64) as usize;
}

/// This macro implements the [`VarInt`] for signed and unsiged types without having
/// to explicitly implement the [`VarInt`] for every number type.
macro_rules! impl_varint {
    ($t:ty, unsigned) => {
        impl Decode for VarInt<$t> {
            fn decode(mut read: impl Read) -> Result<Self>
            where
                Self: Sized,
            {
                let mut result: u64 = 0;
                for times_shift in 0..Self::MAX_BYTES_UNSIGNED {
                    let byte = read.read_u8()?;
                    let msb_dropped = byte & 0b0111_1111;
                    result |= (msb_dropped as u64) << times_shift * 7;

                    if byte & 0b1000_0000 == 0 {
                        return Ok(VarInt::from(result as $t));
                    }
                }

                Err(Error::VarIntError)
            }
        }

        impl Encode for VarInt<$t> {
            fn encode(&self, mut write: impl Write) -> Result<()> {
                let mut n = self.value as u64;

                while n >= 0x80 {
                    write.write_u8(0b1000_0000 | (n as u8))?;
                    n >>= 7;
                }
                write.write_u8(n as u8)?;

                Ok(())
            }

            fn encode_len(&self) -> usize {
                required_encoded_size_unsigned(self.value as u64)
            }
        }

        impl From<$t> for VarInt<$t> {
            fn from(i: $t) -> Self {
                VarInt::<$t> { value: i }
            }
        }

        impl From<VarInt<$t>> for $t {
            fn from(i: VarInt<$t>) -> Self {
                i.value
            }
        }
    };
    ($t:ty, signed) => {
        impl Decode for VarInt<$t> {
            fn decode(mut read: impl Read) -> Result<Self>
            where
                Self: Sized,
            {
                let mut result: u64 = 0;
                for times_shift in 0..Self::MAX_BYTES_SIGNED {
                    let byte = read.read_u8()?;
                    let msb_dropped = byte & 0b0111_1111;
                    result |= (msb_dropped as u64) << times_shift * 7;

                    if byte & 0b1000_0000 == 0 {
                        return Ok(VarInt::from(zigzag_decode(result) as $t));
                    }
                }

                Err(Error::VarIntError)
            }
        }

        impl Encode for VarInt<$t> {
            fn encode(&self, mut write: impl Write) -> Result<()> {
                let mut n: u64 = zigzag_encode(self.value as i64);

                while n >= 0x80 {
                    write.write_u8(0b1000_0000 | (n as u8))?;
                    n >>= 7;
                }

                write.write_u8(n as u8)?;

                Ok(())
            }

            fn encode_len(&self) -> usize {
                required_encoded_space_signed(self.value as i64)
            }
        }

        impl From<$t> for VarInt<$t> {
            fn from(value: $t) -> Self {
                VarInt::<$t> { value }
            }
        }

        impl From<VarInt<$t>> for $t {
            fn from(varint: VarInt<$t>) -> Self {
                varint.value
            }
        }
    };
}

impl_varint!(usize, unsigned);
impl_varint!(u64, unsigned);
impl_varint!(u32, unsigned);
impl_varint!(u16, unsigned);
impl_varint!(u8, unsigned);

impl_varint!(isize, signed);
impl_varint!(i64, signed);
impl_varint!(i32, signed);
impl_varint!(i16, signed);
impl_varint!(i8, signed);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::serialization::{Decode, Encode};
    use std::fmt::Debug;

    fn test_generic<T>(input: T) -> Result<()>
    where
        T: Encode + Decode + Debug + PartialEq,
    {
        let mut encoded: Vec<u8> = Vec::new();
        input.encode(&mut encoded)?;
        let decoded = T::decode(&mut encoded.as_slice())?;

        assert_eq!(input, decoded);
        assert_eq!(input.encode_len(), decoded.encode_len());
        assert_eq!(input.encode_len(), encoded.len());

        Ok(())
    }

    #[test]
    fn test_primitives() -> Result<()> {
        // bools
        test_generic(false)?;
        test_generic(true)?;

        // Unsigned integers
        test_generic(u8::MAX)?;
        test_generic(u16::MAX)?;
        test_generic(u32::MAX)?;
        test_generic(u64::MAX)?;

        // Signed integers
        test_generic(i8::MAX)?;
        test_generic(i16::MAX)?;
        test_generic(i32::MAX)?;
        test_generic(i64::MAX)?;

        // Floats
        test_generic(f32::MAX)?;
        test_generic(f64::MAX)?;

        // Varint
        test_generic(VarInt::from(10))?;
        test_generic(VarInt::from(10))?;
        test_generic(VarInt::from(-10))?;
        test_generic(VarInt::from(u64::MAX))?;

        Ok(())
    }

    #[test]
    fn test_complex() -> Result<()> {
        // Arrays
        test_generic([u8::MAX; 4])?;
        test_generic([u16::MAX; 4])?;
        test_generic([u32::MAX; 4])?;
        test_generic([u64::MAX; 4])?;

        // Strings
        test_generic("SPL ".to_string())?;

        // Vectors
        test_generic(vec![u8::MAX; 4])?;
        test_generic(vec![u16::MAX; 4])?;
        test_generic(vec![u32::MAX; 4])?;
        test_generic(vec![u64::MAX; 4])?;

        // Test array of Varints
        test_generic([VarInt::from(i32::MAX); 4])?;
        test_generic([VarInt::from(i32::MIN); 4])?;
        test_generic([VarInt::from(u32::MAX); 4])?;
        test_generic([VarInt::from(u32::MIN); 4])?;

        Ok(())
    }

    fn test_varint_length_signed(value: VarInt<i32>, expected_encode_len: usize) -> Result<()> {
        let mut buf: Vec<u8> = Vec::new();
        value.encode(&mut buf)?;

        assert_eq!(
            buf.len(),
            expected_encode_len,
            "VarInt wasn't encoded properly, expected length: {}, got: {}",
            expected_encode_len,
            buf.len()
        );

        assert_eq!(
            value.encode_len(),
            expected_encode_len,
            "VarInt wrong `encode_len`, expected length: {}, got: {}",
            expected_encode_len,
            value.encode_len()
        );

        Ok(())
    }

    fn test_varint_length_unsigned(value: VarInt<u32>, expected_encode_len: usize) -> Result<()> {
        let mut buf: Vec<u8> = Vec::new();
        value.encode(&mut buf)?;

        assert_eq!(
            buf.len(),
            expected_encode_len,
            "VarInt wasn't encoded properly, expected length: {}, got: {}",
            expected_encode_len,
            buf.len()
        );

        assert_eq!(
            value.encode_len(),
            expected_encode_len,
            "VarInt wrong `encode_len`, expected length: {}, got: {}",
            expected_encode_len,
            value.encode_len()
        );

        Ok(())
    }

    #[test]
    fn test_varint() -> Result<()> {
        // Test positive values.
        test_varint_length_unsigned(VarInt::from(10), 1)?;
        test_varint_length_unsigned(VarInt::from(127), 1)?;
        test_varint_length_unsigned(VarInt::from(128), 2)?;
        test_varint_length_unsigned(VarInt::from(255), 2)?;
        test_varint_length_unsigned(VarInt::from(2u32.pow(2 * 8)), 3)?;
        test_varint_length_unsigned(VarInt::from(u32::MAX), 5)?;
        test_varint_length_unsigned(VarInt::from(u32::MAX), VarInt::<u32>::MAX_BYTES_UNSIGNED)?;

        // Test negative values.
        test_varint_length_signed(VarInt::from(10), 1)?;
        test_varint_length_signed(VarInt::from(-10), 1)?;
        test_varint_length_signed(VarInt::from(63), 1)?;
        test_varint_length_signed(VarInt::from(64), 2)?;
        test_varint_length_signed(VarInt::from(-64), 1)?;
        test_varint_length_signed(VarInt::from(-65), 2)?;
        test_varint_length_signed(VarInt::from(255), 2)?;
        test_varint_length_signed(VarInt::from(-255), 2)?;
        test_varint_length_signed(VarInt::from(i32::MAX), 5)?;
        test_varint_length_signed(VarInt::from(i32::MIN), 5)?;

        test_varint_length_signed(VarInt::from(i32::MIN), VarInt::<i32>::MAX_BYTES_SIGNED)?;
        test_varint_length_signed(VarInt::from(i32::MAX), VarInt::<i32>::MAX_BYTES_SIGNED)?;

        Ok(())
    }
}
