use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::*;

use crate::error;

pub fn derive(item: TokenStream) -> TokenStream {
    let DeriveInput {
        vis, ident, data, ..
    } = parse_macro_input!(item as DeriveInput);

    let ident_access = Ident::new(&format!("{}Access", ident), Span::call_site());

    let fields: Vec<_> = match &data {
        Data::Struct(DataStruct {
            fields: Fields::Named(FieldsNamed { named, .. }),
            ..
        }) => named.iter().map(|x| x.ident.clone().unwrap()).collect(),
        _ => {
            return error(
                &ident,
                "Data can only be derived from structs with named fields",
            )
        }
    };

    TokenStream::from(quote! {
        impl ::tyr::data::Data for #ident {
            type Access = #ident_access;
        }

        #[derive(Default)]
        #vis struct #ident_access {
            #(pub #fields: ::tyr::data::AccessMode),*
        }

        impl ::tyr::data::Access for #ident_access {
            fn conflicts_with(&self, other: &Self) -> bool {
                #(self.#fields.conflicts_with(other.#fields))|*
            }
        }
    })
}
