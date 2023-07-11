use proc_macro::TokenStream;

mod system;

#[proc_macro_attribute]
pub fn system(_args: TokenStream, item: TokenStream) -> proc_macro::TokenStream {
    system::system(item)
}
