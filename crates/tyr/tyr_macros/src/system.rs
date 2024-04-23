use std::collections::HashSet;

use proc_macro::TokenStream;
use proc_macro2::Ident;
use quote::{quote, ToTokens};
use syn::{
    parse_macro_input, parse_quote, visit_mut::VisitMut, Attribute, FnArg, ItemFn, Pat, PatIdent,
    PatTuple, PatType, Type, TypeReference, TypeTuple,
};

#[cfg(nightly)]
use syn::spanned::Spanned;

/// An argument in a system.
struct SystemArg {
    attrs: Vec<Attribute>,
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

impl ArgTransformerVisitor {
    fn visit_pat_ty_mut(&mut self, attrs: &Vec<Attribute>, mut pat: &mut Pat, mut ty: &mut Type) {
        match (&mut pat, &mut ty) {
            (Pat::Tuple(PatTuple { elems, .. }), Type::Tuple(TypeTuple { elems: types, .. })) => {
                // recursively visit each tuple element, transforming it in place
                for (pat, ty) in elems.iter_mut().zip(types.iter_mut()) {
                    self.visit_pat_ty_mut(attrs, pat, ty);
                }
            }
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
                let mutable = mutability.is_some();

                // save argument information
                self.args.push(SystemArg {
                    attrs: attrs.clone(),
                    mutable,
                    ident: ident.clone(),
                });

                // transform argument name to contain the `mut` keyword, if it's mutable
                *pat = match mutable {
                    true => parse_quote! { mut #ident },
                    false => parse_quote! { #ident },
                };

                // transform argument type to be a `Res` or `ResMut` type
                *ty = match mutable {
                    true => parse_quote! { ::tyr::ResMut<#elem> },
                    false => parse_quote! { ::tyr::Res<#elem> },
                };
            }
            (_, Type::Tuple(TypeTuple { .. })) => {
                self.errors.push(syn::Error::new_spanned(
                    pat,
                    "Tuple arguments must destructure the tuple!",
                ));
            }
            _ => {
                self.errors.push(syn::Error::new_spanned(
                    ty,
                    "Systems can only take references as arguments!",
                ));
            }
        }
    }
}

impl VisitMut for ArgTransformerVisitor {
    fn visit_fn_arg_mut(&mut self, arg: &mut FnArg) {
        if self.skip_first {
            self.skip_first = false;
            return;
        }

        match arg {
            FnArg::Typed(PatType { attrs, pat, ty, .. }) => {
                self.visit_pat_ty_mut(attrs, pat, ty);
            }
            FnArg::Receiver(_) => {
                self.errors.push(syn::Error::new_spanned(
                    arg,
                    "Systems do not support receiver arguments, as they should be implemented as plain functions!",
                ));
            }
        }
    }
}

pub fn system(input: proc_macro::TokenStream, is_startup_system: bool) -> proc_macro::TokenStream {
    let mut input = parse_macro_input!(input as ItemFn);

    if let Err(non_exclusive_mutable_borrow_error) = check_exclusive_mutable_borrow(&input) {
        return non_exclusive_mutable_borrow_error;
    }

    let mut visitor = ArgTransformerVisitor::default();

    // startup sytem specific stuff
    if is_startup_system {
        // don't transform first arg
        visitor.skip_first = true;

        let inputs = &input.sig.inputs;
        if let Some(arg) = inputs.first() {
            // verify the type is `&mut Storage`
            check_mut_storage(&mut visitor, arg);
        } else {
            visitor.errors.push(syn::Error::new_spanned(
                inputs,
                "First argument must be a &mut Storage!",
            ))
        };
    }

    syn::visit_mut::visit_item_fn_mut(&mut visitor, &mut input);

    // automatically deref to a reference at the beginning of a system
    visitor.args.iter().rev().for_each(
        |SystemArg {
             mutable,
             ident,
             attrs,
         }| {
            // adds one of two statements to the beginning of the function block,
            // depending on the mutability of the system argument
            let stmt = if *mutable {
                // Expands to:
                // <attributes> let ident = DerefMut::deref_mut(&mut ident);
                parse_quote! { #(#attrs)* let #ident = std::ops::DerefMut::deref_mut(&mut #ident); }
            } else {
                // Expands to
                // <attibutes> let ident = Deref::deref(&ident);
                parse_quote! { #(#attrs)* let #ident = std::ops::Deref::deref(&#ident); }
            };

            input.block.stmts.insert(0, stmt);
        },
    );

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

    if name_parts.first().is_some_and(|first| first == "&")
        && name_parts.get(1).is_some_and(|second| second == "mut")
    {
        name_parts.insert(2, " ".into());
    }

    name_parts.join("")
}

fn check_mut_storage(visitor: &mut ArgTransformerVisitor, arg: &FnArg) {
    let FnArg::Typed(PatType { ty, .. }) = arg else {
        visitor.errors.push(syn::Error::new_spanned(
            arg,
            "Systems do not support receiver arguments!",
        ));
        return;
    };

    let Type::Reference(TypeReference {
        mutability: Some(_),
        elem,
        ..
    }) = ty.as_ref()
    else {
        visitor.errors.push(syn::Error::new_spanned(
            arg,
            "First argument must be a &mut Storage!",
        ));
        return;
    };

    let Type::Path(path) = elem.as_ref() else {
        visitor.errors.push(syn::Error::new_spanned(
            arg,
            "First argument must be a &mut Storage!",
        ));
        return;
    };

    if *path != parse_quote! { Storage } {
        visitor.errors.push(syn::Error::new_spanned(
            arg,
            "First argument must be a &mut Storage!",
        ));
    };
}
