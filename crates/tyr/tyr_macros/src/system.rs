use std::any::Any;
use std::collections::HashSet;

use proc_macro::TokenStream;
use proc_macro2::Ident;
use quote::{quote, ToTokens};
use syn::token::Mut;
use syn::{
    parse_macro_input, parse_quote, visit_mut::VisitMut, FnArg, ItemFn, Pat, PatIdent, PatType,
    TypeReference,
};
use syn::{Type, TypePath};

#[cfg(nightly)]
use syn::spanned::Spanned;
use tyr_internal::Storage;

/// An argument in a system.
struct SystemArg {
    mutable: bool,
    ident: Ident,
}

/// A visitor that transforms function arguments and records errors.
#[derive(Default)]
struct ArgTransformerVisitor {
    skip_first: bool,
    errors: Vec<syn::Error>,
    args: Vec<SystemArg>,
}

impl VisitMut for ArgTransformerVisitor {
    fn visit_fn_arg_mut(&mut self, arg: &mut FnArg) {
        if self.skip_first {
            self.skip_first = false;
            return;
        }

        match arg {
            FnArg::Typed(PatType { pat, ty, .. }) => match (pat.as_ref(), ty.as_ref()) {
                (
                    Pat::Ident(PatIdent {
                        ident,
                        subpat: None,
                        ..
                    }),
                    Type::Reference(TypeReference {
                        mutability, elem, ..
                    }),
                ) => {
                    let ident = ident.clone();
                    let mutable = mutability.is_some();

                    // substitute function argument
                    *arg = match mutable {
                        true => parse_quote! { mut #ident: ::tyr::ResMut<#elem> },
                        false => parse_quote! { #ident: ::tyr::Res<#elem> },
                    };

                    // save argument information
                    self.args.push(SystemArg { mutable, ident });
                }
                (_, ty) => {
                    self.errors.push(syn::Error::new_spanned(
                        ty,
                        "Systems can only take references as arguments!",
                    ));
                }
            },
            FnArg::Receiver(_) => {
                self.errors.push(syn::Error::new_spanned(
                    arg,
                    "Systems do not support receiver arguments, as they should be implemented as plain functions!",
                ));
            }
        }
    }
}

pub fn system(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let mut input = parse_macro_input!(input as ItemFn);

    if let Err(non_exclusive_mutable_borrow_error) = check_exclusive_mutable_borrow(&input) {
        return non_exclusive_mutable_borrow_error;
    }

    let mut visitor = ArgTransformerVisitor::default();
    syn::visit_mut::visit_item_fn_mut(&mut visitor, &mut input);

    // automatically deref to a reference at the beginning of a system
    visitor
        .args
        .iter()
        .rev()
        .for_each(|SystemArg { mutable, ident }| {
            // adds one of two statements to the beginning of the function block,
            // depending on the mutability of the system argument
            let stmt = if *mutable {
                // Expands to `let ident = DerefMut::deref_mut(&mut ident);`
                parse_quote! { let #ident = std::ops::DerefMut::deref_mut(&mut #ident); }
            } else {
                // Expands to `let ident = Deref::deref(&ident);`
                parse_quote! { let #ident = std::ops::Deref::deref(&#ident); }
            };

            input.block.stmts.insert(0, stmt);
        });

    let errors: TokenStream = visitor
        .errors
        .iter_mut()
        .fold(proc_macro2::TokenStream::default(), |mut acc, error| {
            acc.extend::<proc_macro2::TokenStream>(error.clone().into_compile_error());
            acc
        })
        .into();

    if !errors.is_empty() {
        return errors;
    }

    quote! {
        #input
    }
    .into()
}

pub fn startup_system(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let mut input = parse_macro_input!(input as ItemFn);

    if let Err(non_exclusive_mutable_borrow_error) = check_exclusive_mutable_borrow(&input) {
        return non_exclusive_mutable_borrow_error;
    }

    let arg = input
        .sig
        .inputs
        .first()
        .unwrap_or_else(|| panic!("Startup systems must have at least one parameter!"));

    match arg {
        FnArg::Receiver(_) => panic!("Systems do not support receiver arguments, as they should be implemented as plain functions!"),
        FnArg::Typed(PatType { ty, .. }) => match ty.as_ref() {
            Type::Reference(TypeReference { mutability: Some(_), elem, .. }) => match elem.as_ref() {
                Type::Path(p) if *p == parse_quote! { Storage } => (),
                _ => panic!("First argument must be a &mut Storage!"),
            },
            _ => panic!("First argument must be a &mut Storage!"),
        },
    };

    // if arg != std::any::TypeId::of::<&mut Storage>() {
    //     panic!(
    //         "First parameter in a startup system must be &mut Storage!, {:?}, {:?} != {:?}",
    //         arg,
    //         arg,
    //         std::any::TypeId::of::<&mut Storage>()
    //     );
    // }

    let mut visitor = ArgTransformerVisitor::default();
    visitor.skip_first = true;
    syn::visit_mut::visit_item_fn_mut(&mut visitor, &mut input);

    // automatically deref to a reference at the beginning of a system
    visitor
        .args
        .iter()
        .rev()
        .for_each(|SystemArg { mutable, ident }| {
            // adds one of two statements to the beginning of the function block,
            // depending on the mutability of the system argument
            let stmt = if *mutable {
                // Expands to `let ident = DerefMut::deref_mut(&mut ident);`
                parse_quote! { let #ident = std::ops::DerefMut::deref_mut(&mut #ident); }
            } else {
                // Expands to `let ident = Deref::deref(&ident);`
                parse_quote! { let #ident = std::ops::Deref::deref(&#ident); }
            };

            input.block.stmts.insert(0, stmt);
        });

    let errors: TokenStream = visitor
        .errors
        .iter_mut()
        .fold(proc_macro2::TokenStream::default(), |mut acc, error| {
            acc.extend::<proc_macro2::TokenStream>(error.clone().into_compile_error());
            acc
        })
        .into();

    if !errors.is_empty() {
        return errors;
    }

    quote! {
        #input
    }
    .into()
}

fn check_exclusive_mutable_borrow(input: &ItemFn) -> Result<(), TokenStream> {
    let mut types = HashSet::new();

    for input in input.sig.inputs.iter() {
        let ty = match input {
            FnArg::Typed(ty) => &ty.ty,
            FnArg::Receiver(_) => continue,
        };

        let ty_string = quote! { #ty }.to_string();

        if !types.insert(ty_string.clone()) {
            return Err(syn::Error::new_spanned(
                ty,
                format!(
                    "Resource `{}` is borrowed mutably more than once, this is not allowed!",
                    get_ty_string(ty)
                ),
            )
            .to_compile_error()
            .into());
        }
    }

    Ok(())
}

fn get_ty_string(ty: &Type) -> String {
    #[cfg(nightly)]
    // need nightly compiler for this to work, until [`Span::join`] is stabilised.
    return ty.span().source_text().unwrap_or(build_ty_string(ty));

    build_ty_string(ty)
}

fn build_ty_string(ty: &Type) -> String {
    let iter = ty.to_token_stream().into_iter();
    let mut name_parts: Vec<_> = iter
        .map(|token| token.span().source_text().unwrap_or_default())
        .collect();

    if name_parts.first().is_some_and(|first| first == "&") {
        if name_parts.get(1).is_some_and(|second| second == "mut") {
            name_parts.insert(2, " ".into());
        } else {
            name_parts.insert(1, " ".into());
        }
    }
    name_parts.join("")
}
