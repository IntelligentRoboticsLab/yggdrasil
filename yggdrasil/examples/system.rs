//! A minimal example that shows off a system that prints hello every second

use std::{thread, time::Duration};
use yggdrasil::prelude::*;

/// This is a simple system that prints hello to the terminal, and sleeps for a second
///
/// Systems are normal Rust functions with a couple of small differences
/// - Its parameters must always be a reference `&T` or `&mut T`, never an owned `T`.
/// - The return type must be `Result<()>`
/// - On top of the function we place a macro `#[system]`. This performs some magic that you will not need to know about for now.
/// ## Example:
/// ```ignore
/// // Okay!
/// #[system]
/// fn system_1() -> Result<()> {}
///
/// // Not okay! (missing `#[system]`)
/// fn system_2(foo: &Foo, bar: &Bar) -> Result<()> {}
///
/// // Okay!
/// #[system]
/// fn system_1(foo: &mut Foo, bar: &Bar, baz: &mut Baz) -> Result<()> {}
///
/// // Not okay! (not taking `foo` by reference)
/// #[system]
/// fn system_1(foo: Foo, bar: &Bar) -> Result<()> {}
/// ```
#[system]
fn say_hi() -> Result<()> {
    println!("Hello, Robocup!");

    // Sleep for 1 second, so we don't spam hello messages as fast as we can!
    thread::sleep(Duration::from_secs(1));

    Ok(())
}

fn main() -> Result<()> {
    App::new()
        // Add our system to the app
        .add_system(say_hi)
        // Once we call `App::run()`, systems will start start executing in a loop
        .run()
}
