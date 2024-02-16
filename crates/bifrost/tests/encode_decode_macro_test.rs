use bifrost::{
    serialization::{Decode, Encode},
    Result,
};
use std::fmt::Debug;

fn test_encode_decode<T>(input: &T) -> Result<()>
where
    T: Encode + Decode + Debug + PartialEq,
{
    let mut encoded: Vec<u8> = Vec::new();
    input.encode(&mut encoded)?;
    let decoded = T::decode(&mut encoded.as_slice())?;

    assert_eq!(input, &decoded);
    assert_eq!(input.encode_len(), decoded.encode_len());
    assert_eq!(input.encode_len(), encoded.len());

    Ok(())
}

fn test_encode_discriminant<T>(input: T, expected_discriminant: i16) -> Result<()>
where
    T: Encode + Decode + Debug + PartialEq,
{
    let mut encode_buf = Vec::<u8>::new();
    input.encode(&mut encode_buf)?;

    assert!(
        encode_buf.len() >= 2,
        "Discriminant is smaller than expected",
    );

    let decoded_discriminant = i16::from_le_bytes([encode_buf[0], encode_buf[1]]);
    assert_eq!(decoded_discriminant, expected_discriminant);

    Ok(())
}

#[test]
fn test_encode_decode_struct_named_fields() -> Result<()> {
    #[derive(Encode, Decode, Debug, PartialEq)]
    pub struct TestStruct {
        pub my_u8: u8,
        pub my_arr: [u8; 4],
    }

    let test_struct = TestStruct {
        my_u8: 1,
        my_arr: [0; 4],
    };
    test_encode_decode(&test_struct)?;

    Ok(())
}

#[test]
fn test_encode_decode_struct_unnamed_fields() -> Result<()> {
    #[derive(Encode, Decode, Debug, PartialEq)]
    pub struct TestStruct(u8, [u8; 4]);

    let test_struct = TestStruct(1, [0; 4]);
    test_encode_decode(&test_struct)?;

    Ok(())
}

#[test]
fn test_generic_struct_encode_decode() -> Result<()> {
    #[derive(Encode, Decode, Debug, PartialEq)]
    pub struct GenericStruct<T: Decode + Encode, S, const N: usize>
    where
        S: Decode + Encode,
    {
        foo: T,
        bar: S,
    }

    let test_generic_struct: GenericStruct<u8, u32, 10> = GenericStruct { foo: 1, bar: 10 };
    test_encode_decode(&test_generic_struct)?;

    Ok(())
}

#[test]
fn test_enum_encode_decode() -> Result<()> {
    #[derive(Encode, Decode, Debug, PartialEq)]
    pub enum TestEnum {
        First(u8),
        Second(u16),
        Third(u16, u16),
        Fourth,
        Fifth { x: f32, y: f32 },
    }

    let variant1 = TestEnum::First(8);
    let variant2 = TestEnum::First(16);
    let variant3 = TestEnum::Third(16, 16);
    let variant4 = TestEnum::Fourth;
    let variant5 = TestEnum::Fifth { x: 14.0, y: 15.0 };

    test_encode_decode(&variant1)?;
    test_encode_decode(&variant2)?;
    test_encode_decode(&variant3)?;
    test_encode_decode(&variant4)?;
    test_encode_decode(&variant5)?;

    Ok(())
}

#[test]
fn test_enum_generic_encode_decode() -> Result<()> {
    #[derive(Encode, Decode, Debug, PartialEq)]
    pub enum GenericEnum<T: Decode + Encode, S>
    where
        S: Decode + Encode,
    {
        Foo(T),
        Bar(S),
    }

    let generic_variant1: GenericEnum<u8, i32> = GenericEnum::Foo(8);
    let generic_variant2: GenericEnum<u8, i32> = GenericEnum::Bar(32);

    test_encode_decode(&generic_variant1)?;
    test_encode_decode(&generic_variant2)?;

    Ok(())
}

#[test]
fn test_enum_custom_discriminant_encode_decode() -> Result<()> {
    const DISCRIMINANT_SIXTH: usize = 8;

    const DISCRIMINANT_FIRST: i16 = 0;
    const DISCRIMINANT_SECOND: i16 = 3;
    const DISCRIMINANT_THIRD: i16 = 2;
    const DISCRIMINANT_FOURTH: i16 = 1;
    const DISCRIMINANT_FIFTH: i16 = 4 + 8;

    #[derive(Encode, Decode, Debug, PartialEq)]
    #[repr(C, i16, align(4))]
    pub enum TestEnum {
        First(u8) = DISCRIMINANT_FIRST,
        Second(u16) = DISCRIMINANT_SECOND,
        Third(u16, u16) = DISCRIMINANT_THIRD,
        Fourth = DISCRIMINANT_FOURTH,
        Fifth { x: f32, y: f32 } = DISCRIMINANT_FIFTH,
        Sixth = DISCRIMINANT_SIXTH as i16,
        Seventh,
    }

    let variant_first = TestEnum::First(8);
    let variant_second = TestEnum::Second(16);
    let variant_third = TestEnum::Third(16, 16);
    let variant_fourth = TestEnum::Fourth;
    let variant_fifth = TestEnum::Fifth { x: 14.0, y: 15.0 };
    let variant_sixth = TestEnum::Sixth;
    let variant_seventh = TestEnum::Seventh;

    test_encode_decode(&variant_first)?;
    test_encode_decode(&variant_second)?;
    test_encode_decode(&variant_third)?;
    test_encode_decode(&variant_fourth)?;
    test_encode_decode(&variant_fifth)?;
    test_encode_decode(&variant_sixth)?;
    test_encode_decode(&variant_seventh)?;

    test_encode_discriminant(variant_first, DISCRIMINANT_FIRST)?;
    test_encode_discriminant(variant_second, DISCRIMINANT_SECOND)?;
    test_encode_discriminant(variant_third, DISCRIMINANT_THIRD)?;
    test_encode_discriminant(variant_fourth, DISCRIMINANT_FOURTH)?;
    test_encode_discriminant(variant_fifth, DISCRIMINANT_FIFTH)?;
    test_encode_discriminant(variant_sixth, DISCRIMINANT_SIXTH as i16)?;
    test_encode_discriminant(variant_seventh, (DISCRIMINANT_SIXTH + 1) as i16)?;

    Ok(())
}
