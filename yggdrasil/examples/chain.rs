//! A minimal example that shows off a system that prints hello every second

use std::{thread, time::Duration};
use yggdrasil::prelude::*;

#[system]
fn first() -> Result<()> {
    println!("first");

    // Sleep for 1 second, so we don't spam hello messages as fast as we can!
    thread::sleep(Duration::from_secs(1));

    Ok(())
}

#[system]
fn second() -> Result<()> {
    println!("second");

    // Sleep for 1 second, so we don't spam hello messages as fast as we can!
    thread::sleep(Duration::from_secs(1));

    Ok(())
}

#[system]
fn third() -> Result<()> {
    println!("third");

    // Sleep for 1 second, so we don't spam hello messages as fast as we can!
    thread::sleep(Duration::from_secs(1));

    Ok(())
}

#[system]
fn fourth() -> Result<()> {
    println!("fourth");

    // Sleep for 1 second, so we don't spam hello messages as fast as we can!
    thread::sleep(Duration::from_secs(1));

    Ok(())
}

#[system]
fn fifth() -> Result<()> {
    println!("fifth");
    thread::sleep(Duration::from_secs(1));
    Ok(())
}

#[system]
fn always_before() -> Result<()> {
    println!("before");
    Ok(())
}

#[system]
fn always_after() -> Result<()> {
    println!("after");
    Ok(())
}

fn main() -> Result<()> {
    App::new()
        .add_system(always_before)
        .add_system_chain((first.after(always_before), second, third, fourth, fifth))
        .add_system(always_after.after(fifth))
        .run()
}
