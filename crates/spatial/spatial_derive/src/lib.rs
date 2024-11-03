#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(dead_code)]

use std::collections::HashMap;

use petgraph::{
    graph::{DiGraph, NodeIndex},
    visit::IntoNodeReferences,
};
use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse_macro_input, Data, DeriveInput, Error, Field, Fields, GenericArgument, Ident, Path,
    PathArguments, Type,
};

#[proc_macro_derive(Transform)]
pub fn macro_derive_transform(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    match derive_transform(input) {
        Ok(stream) => stream,
        Err(error) => error.to_compile_error().into(),
    }
}

fn derive_transform(input: DeriveInput) -> Result<TokenStream, Error> {
    let fields = match input.data {
        Data::Struct(data) => data.fields,
        _ => {
            return Err(Error::new_spanned(
                input,
                "`Transform` can only be derived for structs.",
            ))
        }
    };

    let named = match fields {
        Fields::Named(fields) => fields.named,
        _ => {
            return Err(Error::new_spanned(
                fields,
                "`Transform` can only be derived for structs with named fields.",
            ))
        }
    };

    let name = input.ident;

    let graph = build_transform_graph(named.iter())?;
    Ok(implement_transforms(&name, graph).into())
}

fn build_transform_graph<'a>(
    fields: impl Iterator<Item = &'a Field>,
) -> Result<DiGraph<&'a Path, (&'a Ident, &'a Type, bool)>, Error> {
    let mut graph = DiGraph::new();
    let mut spaces = HashMap::new();

    for field in fields {
        let (s1, s2) = match infer_spaces_from_type(&field.ty) {
            Some(spaces) => spaces,
            None => {
                return Err(Error::new_spanned(
                    field.ty.clone(),
                    "Cannot infer spaces from type.",
                ))
            }
        };
        let s1 = *spaces.entry(s1).or_insert_with(|| graph.add_node(s1));
        let s2 = *spaces.entry(s2).or_insert_with(|| graph.add_node(s2));

        let ident = field.ident.as_ref().unwrap();

        graph.add_edge(s1, s2, (ident, &field.ty, false));
        graph.add_edge(s2, s1, (ident, &field.ty, true));
    }

    Ok(graph)
}

fn infer_spaces_from_type(ty: &Type) -> Option<(&Path, &Path)> {
    let segment = match ty {
        Type::Path(path) => path.path.segments.last().unwrap(),
        _ => return None,
    };

    let args = match &segment.arguments {
        PathArguments::AngleBracketed(args) => &args.args,
        _ => return None,
    };

    let mut args = args.iter().rev();
    let s2 = args.next()?;
    let s1 = args.next()?;

    let s1 = match s1 {
        GenericArgument::Type(Type::Path(path)) => &path.path,
        _ => return None,
    };

    let s2 = match s2 {
        GenericArgument::Type(Type::Path(path)) => &path.path,
        _ => return None,
    };

    Some((s1, s2))
}

fn implement_transforms(
    name: &Ident,
    graph: DiGraph<&Path, (&Ident, &Type, bool)>,
) -> proc_macro2::TokenStream {
    let mut stream = proc_macro2::TokenStream::new();

    for s1 in graph.node_references() {
        for s2 in graph.node_references() {
            if let Some((nodes, edges)) = find_transform_path(&graph, s1.0, s2.0) {
                let (s1, s2) = (s1.1, s2.1);

                let mut bounds: Vec<_> = nodes.iter().map(|node| {
                    quote!(#node: ::spatial::space::Space + ::spatial::space::SpaceOver<T>)
                }).collect();

                bounds.extend(edges.iter().map(|(a, b, field, ty, inverse)| {
                    if *inverse {
                        quote!(#ty: ::spatial::transform::InverseTransform<T, T, #b, #a>)
                    } else {
                        quote!(#ty: ::spatial::transform::Transform<T, T, #a, #b>)
                    }
                }));

                let transforms: Vec<_> = edges
                    .iter()
                    .map(|(a, b, field, ty, inverse)| {
                        if *inverse {
                            quote!(let x = self.#field.inverse_transform(&x);)
                        } else {
                            quote!(let x = self.#field.transform(&x);)
                        }
                    })
                    .collect();

                if edges.is_empty() {
                    stream.extend(quote! {
                        #[automatically_derived]
                        impl<T> ::spatial::transform::Transform<T, T, #s1, #s2> for #name where T: Clone, #(#bounds),* {
                            fn transform(&self, x: &::spatial::space::InSpace<T, #s1>) -> ::spatial::space::InSpace<T, #s2> {
                                x.clone()
                            }
                        }
                    });
                } else {
                    stream.extend(quote! {
                        #[automatically_derived]
                        impl<T> ::spatial::transform::Transform<T, T, #s1, #s2> for #name where #(#bounds),* {
                            fn transform(&self, x: &::spatial::space::InSpace<T, #s1>) -> ::spatial::space::InSpace<T, #s2> {
                                use ::spatial::transform::{Transform, InverseTransform};
                                #(#transforms)*
                                x
                            }
                        }
                    });
                }
            }
        }
    }

    stream
}

fn find_transform_path<'a>(
    graph: &'a DiGraph<&Path, (&'a Ident, &'a Type, bool)>,
    s1: NodeIndex,
    s2: NodeIndex,
) -> Option<Route<'a>> {
    let (_, nodes) = petgraph::algo::astar::astar(graph, s1, |g| g == s2, |_| 1, |_| 0)?;

    let edges = nodes
        .iter()
        .zip(nodes.iter().skip(1))
        .map(|(a, b)| {
            let edge = graph.find_edge(*a, *b).unwrap();

            let a = *graph.node_weight(*a).unwrap();
            let b = *graph.node_weight(*b).unwrap();
            let (field, ty, inverse) = *graph.edge_weight(edge).unwrap();
            (a, b, field, ty, inverse)
        })
        .collect();

    let nodes = nodes
        .iter()
        .map(|node| *graph.node_weight(*node).unwrap())
        .collect();

    Some((nodes, edges))
}

type Route<'a> = (
    Vec<&'a Path>,
    Vec<(&'a Path, &'a Path, &'a Ident, &'a Type, bool)>,
);
