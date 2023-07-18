use proc_macro::TokenStream;
use quote::{quote, quote_spanned};
use syn::{parse::Parse, parse_macro_input, Ident, Token, Type};

struct Wrap {
    ident: Ident,
    ty: Type,
}

impl Parse for Wrap {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let ident: Ident = input.parse()?;
        input.parse::<Token![,]>()?;
        let ty: Type = input.parse()?;

        Ok(Wrap { ident, ty })
    }
}

pub fn wrap(input: TokenStream) -> proc_macro::TokenStream {
    let Wrap { ident, ty } = parse_macro_input!(input as Wrap);
    let ty_string = quote! { #ty }.to_string();
    let doc = format!(" Wrapper struct for [`{ty_string}`].\n\n This allows using [`{ty_string}`] as resource by disambiguating the [`TypeId`](`std::any::TypeId`).");

    quote_spanned! {ident.span() =>
        #[doc = #doc]
        #[derive(Debug, Default)]
        pub struct #ident (#ty);

        impl std::ops::Deref for #ident {
            type Target = #ty;

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
