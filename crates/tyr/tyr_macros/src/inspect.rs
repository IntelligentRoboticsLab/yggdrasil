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

            fn to_json(&self) -> ::serde_json::Value {
                ::serde_json::to_value(self)
                    .expect(concat!("Unable to serialize `", stringify!(#ident), "` to JSON."))
            }

            fn try_update_from_json(&mut self, json: ::serde_json::Value) {
                if let Ok(data) = ::serde_json::from_value(json) {
                    *self = data;
                }
            }
        }
    }
    .into()
}
