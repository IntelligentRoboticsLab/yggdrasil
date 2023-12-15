use quote::quote;

use syn::{Attribute, Expr, Meta, Variant};

use proc_macro2::TokenStream;

/// Calculate the number of bytes required to hold the integer value of `value`.
fn calc_bytes_required(value: usize) -> usize {
    let mut num_of_bytes: u32 = 1;

    loop {
        if value < usize::pow(2, num_of_bytes * 8) {
            return num_of_bytes as usize;
        }

        num_of_bytes += 1;
    }
}

/// Try to extract an integer type from `input`.
fn extract_integer_type(input: &proc_macro2::TokenStream) -> Option<&'static str> {
    let valid_types = [
        "u8", "u16", "u32", "u64", "u128", "i8", "i16", "i32", "i64", "i128",
    ];
    let input_str = input.to_string();

    valid_types
        .into_iter()
        .find(|&valid_type| input_str.contains(valid_type))
}

/// Calculate the size of the discriminant in bytes.
///
/// If there is an attribute list in `attributes` whose ident equals `repr`,
/// this function will check if the list contains an integer type and if it does,
/// use the size of that type as the discriminant size.
///
/// If there is no such attribute, this function will calculate the number of bytes required
/// to hold `num_variants`.
pub fn calculate_variant_discriminant_byte_size<'a>(
    num_variants: usize,
    attributes: &mut impl Iterator<Item = &'a Attribute>,
) -> proc_macro2::TokenStream {
    if let Some(valid_type) = attributes.find_map(|attribute| match &attribute.meta {
        Meta::List(meta_list) => {
            if meta_list.path.is_ident("repr") {
                extract_integer_type(&meta_list.tokens)
            } else {
                None
            }
        }
        _ => None,
    }) {
        let valid_type: Expr = syn::parse_str(valid_type).unwrap();
        quote! { std::mem::size_of::<#valid_type>() }
    } else {
        let bytes_required = calc_bytes_required(num_variants);
        quote! { #bytes_required }
    }
}

/// Calculate the discriminant for each variant of an enum.
///
/// The discriminants are calculated according to Rust's [discriminant rules](https://doc.rust-lang.org/reference/items/enumerations.html#custom-discriminant-values-for-fieldless-enumerations)
pub fn calculate_discriminants<'a>(
    variants: impl Iterator<Item = &'a Variant> + 'a,
) -> impl Iterator<Item = (TokenStream, Variant)> + 'a {
    let mut discriminant_counter: TokenStream = quote! { 0 };

    variants.map(move |variant| {
        let variant = variant.clone();
        let discriminant = if let Some((_, lit)) = &variant.discriminant {
            quote! { #lit }
        } else {
            let discriminant_counter_cpy = discriminant_counter.clone();
            quote! { #discriminant_counter_cpy }
        };

        discriminant_counter = quote! { #discriminant + 1 };

        (discriminant, variant)
    })
}
