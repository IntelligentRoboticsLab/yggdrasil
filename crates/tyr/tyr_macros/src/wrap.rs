use proc_macro::TokenStream;
use quote::quote_spanned;
use syn::{parse::Parse, parse_macro_input, Ident, Token};

struct Wrap {
    ident: Ident,
    ty_ident: Ident,
}

impl Parse for Wrap {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let ident: Ident = input.parse()?;
        input.parse::<Token![,]>()?;
        let ty_ident: Ident = input.parse()?;

        Ok(Wrap { ident, ty_ident })
    }
}

pub fn wrap(input: TokenStream) -> proc_macro::TokenStream {
    let Wrap { ident, ty_ident } = parse_macro_input!(input as Wrap);
    let doc = format!(" Wrapper struct for [`{ty_ident}`].\n\n This allows using [`{ty_ident}`] as resource by disambiguating the [`TypeId`](`std::any::TypeId`).");

    quote_spanned! {ident.span() =>
        #[doc = #doc]
        #[derive(Default)]
        pub struct #ident (#ty_ident);

        impl std::ops::Deref for #ident {
            type Target = #ty_ident;

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl std::ops::DerefMut for #ident {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.0
            }
        }
    }
    .into()
}
