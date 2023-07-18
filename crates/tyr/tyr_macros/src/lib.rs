use proc_macro::TokenStream;

mod system;
mod wrap;

/// Macro that performs substitution of parameter types, making the writing of systems more ergonomic.
///
/// [`Res<T>`](../tyr/struct.Res.html) and [`ResMut<T>`](../tyr/struct.ResMut.html) implement [`Deref`](https://doc.rust-lang.org/std/ops/trait.Deref.html) and [`DerefMut`](https://doc.rust-lang.org/std/ops/trait.DerefMut.html) into `&T` and `&mut T`,
/// this macro gets rid of the conversion step visually and reduces mental (and writing) overhead
///
/// ```ignore
/// // A function like this
/// #[system]
/// fn foo(
///     x: &Bar,
///     y: &mut Baz,
/// ) -> Result<()> { Ok(()) }
///
/// // Gets expanded to a function like this
/// fn foo_behind_the_scenes(
///     x: ::tyr::Res<Bar>,
///     mut y: ::tyr::ResMut<Baz>
/// ) -> Result<()> { Ok(()) }
/// ```
///
#[proc_macro_attribute]
pub fn system(_args: TokenStream, item: TokenStream) -> proc_macro::TokenStream {
    system::system(item)
}

/// Macro that generates wrapper structs to use a type as [`Resource`](../tyr/struct.Resource.html) more than once.
///
/// This wrapper struct will implement both [`Deref`](`std::ops::Deref`) and [`DerefMut`](`std::ops::DerefMut`) for the target type, providing a seamless experience.
#[proc_macro]
pub fn wrap(input: TokenStream) -> proc_macro::TokenStream {
    wrap::wrap(input)
}
