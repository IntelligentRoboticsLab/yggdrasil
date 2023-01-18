extern crate proc_macro;

mod data;
mod system;

use proc_macro::TokenStream;

fn error(loc: &impl syn::spanned::Spanned, msg: &'static str) -> TokenStream {
    syn::Error::new(loc.span(), msg).to_compile_error().into()
}



#[proc_macro_derive(Data)]
pub fn derive_data(item: TokenStream) -> TokenStream {
    data::derive(item)
}

#[proc_macro_attribute]
pub fn system(args: TokenStream, item: TokenStream) -> TokenStream {
    system::system(args, item)
}
