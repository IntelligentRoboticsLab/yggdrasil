//! A minimal example that shows off system chains

use std::{thread, time::Duration};
use yggdrasil::prelude::*;

#[system]
fn first() -> Result<()> {
    println!("first");
    thread::sleep(Duration::from_secs(1));
    Ok(())
}

#[system]
fn second() -> Result<()> {
    println!("second");
    thread::sleep(Duration::from_secs(1));
    Ok(())
}

#[system]
fn third() -> Result<()> {
    println!("third");
    thread::sleep(Duration::from_secs(1));
    Ok(())
}

#[system]
fn newline() -> Result<()> {
    println!();
    Ok(())
}

fn main() -> Result<()> {
    App::new()
        // system chains allow you to easily add multiple systems that should run in sequence
        // a system chain is simply a tuple of systems
        .add_system_chain((first, second, third))
        .add_system(newline.after(third))
        .run()
}
