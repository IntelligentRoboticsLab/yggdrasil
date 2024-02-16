use crate::serialization::tools::{
    calculate_discriminants, calculate_variant_discriminant_byte_size,
};

use proc_macro2::TokenStream;

use syn::{
    parse, Attribute, Data, DataEnum, DataStruct, DataUnion, DeriveInput, Error, Fields,
    FieldsNamed, FieldsUnnamed, Ident, Variant,
};

use quote::quote;

pub fn decode(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    match parse(input) {
        Ok(ast) => impl_codec_derive(&ast),
        Err(error) => error.to_compile_error(),
    }
    .into()
}

fn construct_named_struct(fields: &FieldsNamed) -> TokenStream {
    let (field_types, field_idents): (Vec<_>, Vec<_>) = fields
        .named
        .iter()
        .map(|field| (&field.ty, &field.ident))
        .unzip();

    quote! {
        Self {
            #(#field_idents: <#field_types>::decode(&mut read)?,)*
        }
    }
}

fn construct_unnamed_struct(fields: &FieldsUnnamed) -> TokenStream {
    let field_types = fields.unnamed.iter().map(|field| &field.ty);

    quote! {
        Self (
            #(<#field_types>::decode(&mut read)?,)*
        )
    }
}

fn construct_unit_struct() -> TokenStream {
    // A unit struct does not have any fields, so there is nothing to decode.
    quote! {
        Self {}
    }
}

fn decode_struct(data: &DataStruct) -> TokenStream {
    let constructor_arguments = &match &data.fields {
        Fields::Named(fields) => construct_named_struct(fields),
        Fields::Unnamed(fields) => construct_unnamed_struct(fields),
        Fields::Unit => construct_unit_struct(),
    };

    quote! {
        fn decode(mut read: impl std::io::Read) -> bifrost::Result<Self>
        where
            Self: Sized,
        {
            Ok(
                #constructor_arguments
            )
        }
    }
}

fn decode_variant_discriminant(data: &DataEnum, attributes: &[Attribute]) -> TokenStream {
    let num_variants = data.variants.iter().len();
    let variant_discriminant_byte_size =
        calculate_variant_discriminant_byte_size(num_variants, &mut attributes.iter());

    quote! {
        let mut variant_discriminant_buf = [0_u8; std::mem::size_of::<usize>()];
        read.read_exact(&mut variant_discriminant_buf[0..#variant_discriminant_byte_size])?;
        let variant_discriminant: usize = usize::from_le_bytes(variant_discriminant_buf);
    }
}

fn decode_variant_named_fields(
    discriminant: &TokenStream,
    fields: &FieldsNamed,
    ident: &Ident,
) -> TokenStream {
    let (field_types, field_idents): (Vec<_>, Vec<_>) = fields
        .named
        .iter()
        .map(|field| (&field.ty, &field.ident))
        .unzip();

    quote! {
        discriminant if discriminant == (#discriminant) as usize =>
            { Ok(Self::#ident{#(#field_idents: <#field_types>::decode(&mut read)?),*}) },
    }
}

fn decode_variant_unnamed_fields(
    discriminant: &TokenStream,
    fields: &FieldsUnnamed,
    ident: &Ident,
) -> TokenStream {
    let field_types = fields.unnamed.iter().map(|field| &field.ty);

    quote! {
        discriminant if discriminant == (#discriminant) as usize => { Ok(Self::#ident(#(<#field_types>::decode(&mut read)?),*)) },
    }
}

fn decode_variant_unit_fields(discriminant: &TokenStream, ident: &Ident) -> TokenStream {
    quote! {
        discriminant if discriminant == (#discriminant) as usize => { Ok(Self::#ident) },
    }
}

fn decode_variant_fields((discriminant, variant): (TokenStream, Variant)) -> TokenStream {
    let ident = &variant.ident;

    let discriminant = if let Some((_, lit)) = &variant.discriminant {
        quote! { ( #lit as usize ) }
    } else {
        quote! { #discriminant }
    };

    match &variant.fields {
        Fields::Named(fields) => decode_variant_named_fields(&discriminant, fields, ident),
        Fields::Unnamed(fields) => decode_variant_unnamed_fields(&discriminant, fields, ident),
        Fields::Unit => decode_variant_unit_fields(&discriminant, ident),
    }
}

fn decode_variant(enum_ident: &Ident, data: &DataEnum) -> TokenStream {
    let variant_match_arms =
        calculate_discriminants(data.variants.iter()).map(decode_variant_fields);

    quote! {
        match variant_discriminant {
            #(#variant_match_arms)*
            discriminant => Err(bifrost::Error::InvalidVariantDiscriminant(discriminant as usize, stringify!(#enum_ident))),
        }
    }
}

fn decode_enum(enum_ident: &Ident, data: &DataEnum, attributes: &[Attribute]) -> TokenStream {
    let gen_decode_variant_discriminant = decode_variant_discriminant(data, attributes);
    let gen_decode_read = decode_variant(enum_ident, data);

    quote! {
        fn decode(mut read: impl std::io::Read) -> bifrost::Result<Self>
        where
            Self: Sized,
        {
            #gen_decode_variant_discriminant

            #gen_decode_read
        }
    }
}

fn decode_union(data: &DataUnion) -> TokenStream {
    Error::new(
        data.union_token.span,
        "`Decode` cannot be derived for unions.",
    )
    .to_compile_error()
}

fn decode_fn(ast: &DeriveInput, attributes: &[Attribute]) -> TokenStream {
    match &ast.data {
        Data::Struct(data) => decode_struct(data),
        Data::Enum(data) => decode_enum(&ast.ident, data, attributes),
        Data::Union(data) => decode_union(data),
    }
}

fn impl_codec_derive(ast: &DeriveInput) -> TokenStream {
    let type_name = &ast.ident;

    let (template_arguments_with_bounds, template_arguments_without_bounds, template_where_clause) =
        &ast.generics.split_for_impl();

    let decode_fn = decode_fn(ast, &ast.attrs);

    quote! {
        impl #template_arguments_with_bounds bifrost::serialization::Decode
        for #type_name #template_arguments_without_bounds #template_where_clause {
            #decode_fn
        }
    }
}
