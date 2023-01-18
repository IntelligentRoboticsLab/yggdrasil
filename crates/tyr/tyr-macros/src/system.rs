use proc_macro::TokenStream;
use quote::quote;
use syn::*;

use crate::error;

pub fn system(args: TokenStream, item: TokenStream) -> TokenStream {
    let ItemFn { attrs, vis, sig, block } = parse_macro_input!(item as ItemFn);
    let data = parse_macro_input!(args as TypePath);

    let access: Path = parse_str(&format!("{}Access", quote!{ #data })).unwrap();

    let mut exclusive = Vec::new();
    let mut shared = Vec::new();

    let (mut args_id, mut args_mut) = (Vec::new(), Vec::new());


    for input in &sig.inputs {
        match input {
            FnArg::Typed(PatType { pat, ty, .. }) => match (pat.as_ref(), ty.as_ref()) {
                (
                    Pat::Ident(PatIdent { ident, subpat: None, .. }),
                    Type::Reference(TypeReference { mutability, .. }),
                ) => {
                    match mutability {
                        Some(_) => exclusive.push(ident),
                        None => shared.push(ident),
                    }

                    args_id.push(ident);
                    args_mut.push(mutability);
                },
                _ => return error(input, "systems can only take references as arguments"),

            },
            _ => return error(input, "systems cannot take self as argument"),
        }
    }


    let ident = &sig.ident;
    let name = ident.to_string();

    TokenStream::from(quote! {
        #vis fn #ident() -> tyr::system::System<#data> {
            tyr::system::System::new(
                #name.into(),
                #access {
                    #(#exclusive: tyr::data::AccessMode::Exclusive,)*
                    #(#shared: tyr::data::AccessMode::Shared,)*
                    ..Default::default()
                },
                Box::new(|data: *mut #data| {
                    #(#attrs),* #sig #block

                    unsafe {
                        #ident(#(&#args_mut (*data).#args_id),*)
                    }
                })
            )
        }
    })

}
