//! Startup systems are a useful tool for things that need to be initialized once.
//!
//! A startup system runs once when added to the app.

use yggdrasil::prelude::*;

struct A(String);
struct B(String);
struct C();

/// Startup systems differ from normal systems in three main ways
/// The first difference is that they are marked with a different macro
/// e.g. `#[startup_system]` instead of `#[system]`
///
/// The second difference is that the first parameter must always be `&mut Storage`.
/// You can use the storage to add resources to the App.
/// After this first parameter, you can query any resource `T` by using `&T` or `&mut T` like in a normal system.
#[startup_system]
fn first(storage: &mut Storage) -> Result<()> {
    let a = A("a".to_string());
    let b = B("b".to_string());

    storage.add_resource(Resource::new(a))?;
    storage.add_resource(Resource::new(b))?;

    Ok(())
}

/// The second difference is that startup systems run directly when added to the App.
/// This means you can make use of resources that have been added by a previous startup system.
#[startup_system]
fn second(storage: &mut Storage, a: &A, b: &mut B) -> Result<()> {
    b.0 = "🅱️".to_string();

    let c = C();

    println!("{}", a.0);
    println!("{}", b.0);

    storage.add_resource(Resource::new(c))?;

    Ok(())
}

fn main() -> Result<()> {
    App::new()
        .add_startup_system(first)?
        .add_startup_system(second)?
        .run()
}
