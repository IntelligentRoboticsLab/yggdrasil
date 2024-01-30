//! Adding a resource
//!
//! Resources are used to store state and must be added to the app in order to become accessible from systems
//!
//! For each distinct type, there can be only 1 resource in the App. This means that adding two resources of the
//! same type will result in an error!
//!
//! # Example
//!
//! ```ignore
//! fn main() -> Result<()> {
//!     App::new()
//!         .add_resource(String::from("First string")))?
//!         // This will error!
//!         .add_resource(String::from("Second string")))?
//!         .run()
//! }
//! ```
//!
//! This means that for common types like [`String`], you might want to create a [newtype](https://rust-unofficial.github.io/patterns/patterns/behavioural/newtype.html):
//!
//! # Example
//!
//! ```
//! // Example of a newtype that wraps a string
//! // This will be a different type to [`String`], so it's safe to add it to the app.
//! pub struct RobotName(String);
//! ```

use std::{thread, time::Duration};
use yggdrasil::prelude::*;

#[derive(Default)]
enum Greeting {
    #[default]
    Hello,
    Goodbye,
}

impl std::fmt::Display for Greeting {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Greeting::Hello => "Hello",
            Greeting::Goodbye => "Goodbye",
        })
    }
}

struct Thing(String);

impl Thing {
    fn new(inner: impl Into<String>) -> Self {
        Self(inner.into())
    }
}

#[system]
// We are taking a mutable reference (`&mut`) of hello, so we can mutate its state
fn greet(greeting: &mut Greeting, thing: &Thing) -> Result<()> {
    println!("{}, {}!", greeting, thing.0);

    *greeting = match greeting {
        Greeting::Hello => Greeting::Goodbye,
        Greeting::Goodbye => Greeting::Hello,
    };

    thread::sleep(Duration::from_secs(1));

    Ok(())
}

fn main() -> Result<()> {
    App::new()
        // In systems, you can only access resources that have been added to the app
        .add_resource(Resource::new(Thing::new("world")))?
        // If the struct implements [`Default`], you can initialize it like this
        .init_resource::<Greeting>()?
        .add_system(greet)
        .run()
}
