//! A minimal example that shows off the tracing integration with tyr.
//!
//! This example demonstrates the usage of the tracing feature with [`tracing_tracy`],
//! and it expects a running Tracy server to be able to visualize the traces.

use miette::Result;
use std::{thread, time::Duration};
use tracing_subscriber::layer::SubscriberExt;
use tyr::prelude::*;

/// This is a simple system that sleeps for half a second
#[system]
fn half_second() -> Result<()> {
    thread::sleep(Duration::from_millis(500));
    Ok(())
}

/// This is a simple system that sleeps for a second
#[system]
fn one_second() -> Result<()> {
    thread::sleep(Duration::from_secs(1));
    Ok(())
}

/// This is a simple system that sleeps for two seconds
#[system]
fn two_seconds() -> Result<()> {
    thread::sleep(Duration::from_secs(2));
    Ok(())
}

fn main() -> Result<()> {
    // Set up the tracy subscriber
    tracing::subscriber::set_global_default(
        tracing_subscriber::registry().with(tracing_tracy::TracyLayer::default()),
    )
    .expect("setup tracy layer");

    App::new()
        // Add our systems to the app
        .add_system(half_second)
        .add_system(one_second)
        .add_system(two_seconds)
        // Once we call `App::run()`, systems will start start executing in a loop
        .run()
}
