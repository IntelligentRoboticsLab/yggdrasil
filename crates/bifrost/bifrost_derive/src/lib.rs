//! This crate provides bifrost's derive macros.
mod serialization;

use serialization::{decode, encode};

/// Implements a derive macro for the [Encode] trait.
#[proc_macro_derive(Encode)]
pub fn encode(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    encode::encode(input)
}

/// Implements a derive macro for the [Decode] trait.
#[proc_macro_derive(Decode)]
pub fn decode(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    decode::decode(input)
}
