use crate::serialization::tools::{
    calculate_discriminants, calculate_variant_discriminant_byte_size,
};

use syn::{
    parse, Attribute, Data, DataEnum, DataStruct, DataUnion, DeriveInput, Error, Fields,
    FieldsNamed, FieldsUnnamed, Ident, Variant,
};

use quote::{format_ident, quote};

use proc_macro2::TokenStream;

pub fn unions_unsupported_error(union: syn::token::Union) -> TokenStream {
    Error::new(union.span, "Cannot derive `Encode` for unions.").to_compile_error()
}

pub fn encode(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    match parse(input) {
        Ok(ast) => impl_codec_derive(&ast),
        Err(error) => error.to_compile_error(),
    }
    .into()
}

fn encode_named_struct(fields: &FieldsNamed) -> TokenStream {
    let field_names = fields.named.iter().map(|field| &field.ident);

    quote! {
        #(self.#field_names.encode(&mut write)?;)*
    }
}

fn encode_unnamed_struct(fields: &FieldsUnnamed) -> TokenStream {
    let field_ids = fields
        .unnamed
        .iter()
        .enumerate()
        .map(|(id, _field)| syn::Index::from(id));

    quote! {
        #(self.#field_ids.encode(&mut write)?;)*
    }
}

fn encode_unit_struct() -> TokenStream {
    // A unit struct does not have any fields, so there is nothing to encode.
    quote! {}
}

fn encode_struct(data: &DataStruct) -> TokenStream {
    let encode_fn_body = match &data.fields {
        Fields::Named(fields) => encode_named_struct(fields),
        Fields::Unnamed(fields) => encode_unnamed_struct(fields),
        Fields::Unit => encode_unit_struct(),
    };

    quote! {
        fn encode(&self, mut write: impl std::io::Write) -> bifrost::Result<()> {
            #encode_fn_body
            Ok(())
        }
    }
}

fn encode_variant_discriminant(data: &DataEnum, attributes: &[Attribute]) -> TokenStream {
    let encode_variant_discriminant_match_arms =
        calculate_discriminants(data.variants.iter()).map(|(discriminant, variant)| {
            let ident = &variant.ident;

            match &variant.fields {
                Fields::Named(..) => quote! {
                    Self::#ident{..} => (#discriminant) as usize
                },
                Fields::Unnamed(..) => quote! {
                    Self::#ident(..) => (#discriminant) as usize
                },
                Fields::Unit => quote! {
                    Self::#ident => (#discriminant) as usize
                },
            }
        });

    let num_variants = data.variants.iter().len();
    let variant_discriminant_byte_size =
        calculate_variant_discriminant_byte_size(num_variants, &mut attributes.iter());

    quote! {
        let variant_discriminant: usize = match self {
            #(#encode_variant_discriminant_match_arms),*
        };
        write.write_all(&variant_discriminant.to_le_bytes()[0..#variant_discriminant_byte_size])?;
    }
}

fn encode_variant_named_fields(ident: &Ident, fields: &FieldsNamed) -> TokenStream {
    let fields: Vec<_> = fields.named.iter().map(|field| &field.ident).collect();

    quote! {
        Self::#ident{#(#fields),*} => { #(#fields.encode(&mut write)?;)* }
    }
}

fn encode_variant_unnamed_fields(ident: &Ident, fields: &FieldsUnnamed) -> TokenStream {
    let fields: Vec<_> = fields
        .unnamed
        .iter()
        .enumerate()
        .map(|(i, _)| format_ident!("arg{}", i))
        .collect();

    quote! {
        Self::#ident(#(#fields),*) => { #(#fields.encode(&mut write)?;)* }
    }
}

fn encode_variant_unit_fields(ident: &Ident) -> TokenStream {
    quote! {
        Self::#ident => {},
    }
}

fn encode_variant_fields(variant: &Variant) -> TokenStream {
    let ident = &variant.ident;

    match &variant.fields {
        Fields::Named(fields) => encode_variant_named_fields(ident, fields),
        Fields::Unnamed(fields) => encode_variant_unnamed_fields(ident, fields),
        Fields::Unit => encode_variant_unit_fields(ident),
    }
}

fn encode_variant(data: &DataEnum) -> TokenStream {
    let variants_match_arms = data.variants.iter().map(encode_variant_fields);

    quote! {
        match self {
            #(#variants_match_arms)*
        };
    }
}

fn encode_enum(data: &DataEnum, attributes: &[Attribute]) -> TokenStream {
    let encode_variant_discriminant = encode_variant_discriminant(data, attributes);
    let encode_enum_write = encode_variant(data);

    quote! {
        fn encode(&self, mut write: impl std::io::Write) -> bifrost::Result<()> {
            #encode_variant_discriminant
            #encode_enum_write
            Ok(())
        }
    }
}

fn encode_union(data: &DataUnion) -> TokenStream {
    unions_unsupported_error(data.union_token)
}

fn encode_fn(ast: &DeriveInput, attributes: &[Attribute]) -> TokenStream {
    match &ast.data {
        Data::Struct(data) => encode_struct(data),
        Data::Enum(data) => encode_enum(data, attributes),
        Data::Union(data) => encode_union(data),
    }
}

fn encode_len_named_struct(fields: &FieldsNamed) -> TokenStream {
    let field_names = fields.named.iter().map(|field| &field.ident);

    if field_names.len() == 0 {
        quote! { 0 }
    } else {
        quote! { #(self.#field_names.encode_len())+* }
    }
}

fn encode_len_unnamed_struct(fields: &FieldsUnnamed) -> TokenStream {
    let field_ids = fields
        .unnamed
        .iter()
        .enumerate()
        .map(|(id, _field)| syn::Index::from(id));

    quote! {
        #(self.#field_ids.encode_len())+*
    }
}

fn encode_len_unit_struct() -> TokenStream {
    quote! {
        0
    }
}

fn encode_len_struct(data: &DataStruct) -> TokenStream {
    let encode_len_fn_body = match &data.fields {
        Fields::Named(fields) => encode_len_named_struct(fields),
        Fields::Unnamed(fields) => encode_len_unnamed_struct(fields),
        Fields::Unit => encode_len_unit_struct(),
    };

    quote! {
        fn encode_len(&self) -> usize {
            #encode_len_fn_body
        }
    }
}

fn encode_len_named_variant(ident: &Ident, fields: &FieldsNamed) -> TokenStream {
    let fields: Vec<_> = fields.named.iter().map(|field| &field.ident).collect();

    quote! {
        Self::#ident{#(#fields),*} => #(#fields.encode_len())+*
    }
}

fn encode_len_unnamed_variant(ident: &Ident, fields: &FieldsUnnamed) -> TokenStream {
    let fields: Vec<_> = fields
        .unnamed
        .iter()
        .enumerate()
        .map(|(i, _)| format_ident!("arg{}", i))
        .collect();

    quote! {
        Self::#ident(#(#fields),*) => #(#fields.encode_len())+*
    }
}

fn encode_len_enum_variant_unit(ident: &Ident) -> TokenStream {
    quote! {
        Self::#ident => 0
    }
}

fn encode_len_variant(variant: &Variant) -> TokenStream {
    let ident = &variant.ident;

    match &variant.fields {
        Fields::Named(fields) => encode_len_named_variant(ident, fields),
        Fields::Unnamed(fields) => encode_len_unnamed_variant(ident, fields),
        Fields::Unit => encode_len_enum_variant_unit(ident),
    }
}

fn encode_len_enum(data: &DataEnum, attributes: &[Attribute]) -> TokenStream {
    let variant_match_arms = data.variants.iter().map(encode_len_variant);
    let num_variants = data.variants.iter().len();
    let variant_discriminant_byte_size =
        calculate_variant_discriminant_byte_size(num_variants, &mut attributes.iter());

    quote! {
        fn encode_len(&self) -> usize {
            #variant_discriminant_byte_size +
            match self {
                #(#variant_match_arms),*
            }
        }
    }
}

fn encode_len_union(data: &DataUnion) -> TokenStream {
    unions_unsupported_error(data.union_token)
}

fn encode_len_fn(ast: &DeriveInput, attributes: &[Attribute]) -> TokenStream {
    match &ast.data {
        Data::Struct(data) => encode_len_struct(data),
        Data::Enum(data) => encode_len_enum(data, attributes),
        Data::Union(data) => encode_len_union(data),
    }
}

fn impl_codec_derive(ast: &DeriveInput) -> TokenStream {
    let type_name = &ast.ident;

    let (template_arguments_with_bounds, template_arguments_without_bounds, template_where_clause) =
        &ast.generics.split_for_impl();

    let encode_fn = encode_fn(ast, &ast.attrs);
    let encode_len_fn = encode_len_fn(ast, &ast.attrs);

    quote! {
        impl #template_arguments_with_bounds bifrost::serialization::Encode
        for #type_name #template_arguments_without_bounds #template_where_clause {
            #encode_fn

            #encode_len_fn
        }
    }
}
