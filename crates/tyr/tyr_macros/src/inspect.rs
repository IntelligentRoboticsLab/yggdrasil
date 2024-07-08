use quote::quote;
use syn::{parse_macro_input, DeriveInput};

pub fn inspect(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let ident = input.ident;

    quote! {
        impl ::tyr::Inspect for #ident {
            fn name(&self) -> &'static str {
                ::core::any::type_name::<#ident>()
            }

            fn to_json(&self) -> String {
                ::serde_json::to_string(self)
                    .expect(concat!("Unable to serialize `", stringify!(#ident), "` to JSON."))
            }

            fn update_from_json(&mut self, json: &str) {
                *self = ::serde_json::from_str(json)
                    .expect(concat!("Unable to deserialize `", stringify!(#ident), "` from JSON."))
            }
        }
    }
    .into()
}
