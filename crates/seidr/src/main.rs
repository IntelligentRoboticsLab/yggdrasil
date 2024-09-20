//! This example shows how to wrap the Rerun Viewer in your own GUI.
use app::App;
use clap::Parser;
use cli::Cli;
use connection::TcpConnection;
use miette::{IntoDiagnostic, Result};
use re_viewer::external::{eframe, egui, re_log, re_memory};
use std::{
    net::{Ipv4Addr, SocketAddrV4},
    process::exit,
    time::Duration,
};
use yggdrasil::core::control::CONTROL_PORT;

mod app;
mod cli;
mod connection;
mod resource;
mod seidr;
mod style;

const MB_TO_BYTES_MULTIPLIER: u64 = 1_000_000;

// By using `re_memory::AccountingAllocator` Rerun can keep track of exactly how much memory it is using,
// and prune the data store when it goes above a certain limit.
// By using `mimalloc` we get faster allocations.
#[global_allocator]
static GLOBAL: re_memory::AccountingAllocator<mimalloc::MiMalloc> =
    re_memory::AccountingAllocator::new(mimalloc::MiMalloc);

#[tokio::main]
async fn main() -> Result<()> {
    // Direct calls using the `log` crate to stderr. Control with `RUST_LOG=debug` etc.
    re_log::setup_logging();

    let args = Cli::parse();

    tracing::info!("Starting seidr and connection with {}", args.robot_ip);

    // Install handlers for panics and crashes that prints to stderr and send
    // them to Rerun analytics (if the `analytics` feature is on in `Cargo.toml`).
    re_crash_handler::install_crash_handlers(re_viewer::build_info());

    // Listen for TCP connections from Rerun's logging SDKs.
    // There are other ways of "feeding" the viewer though - all you need is a `re_smart_channel::Receiver`.
    let rx = re_sdk_comms::serve(
        &Ipv4Addr::UNSPECIFIED.to_string(),
        re_sdk_comms::DEFAULT_SERVER_PORT,
        Default::default(),
    )
    .into_diagnostic()?;

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_app_id("seidr"),
        ..re_viewer::native::eframe_options(None)
    };

    let memory_limit = if let Some(max_memory) = args.max_mem {
        tracing::info!("Memory limit set to: {} MB", max_memory);
        re_memory::MemoryLimit::from_bytes(MB_TO_BYTES_MULTIPLIER * max_memory)
    } else {
        re_memory::MemoryLimit::from_fraction_of_total(0.75)
    };

    let startup_options = re_viewer::StartupOptions {
        memory_limit,
        ..Default::default()
    };

    let socket_addr = SocketAddrV4::new(args.robot_ip, CONTROL_PORT);

    // Tries to make a connection to the robot address
    let mut connection_attempts = 0;
    let max_connection_attempts = 10;
    let connection = loop {
        match TcpConnection::try_from_ip(socket_addr).await {
            Ok(conn) => break conn,
            Err(err) => {
                tracing::info!(
                    "[{}/{}] Failed to connect: {}. Retrying...",
                    connection_attempts,
                    max_connection_attempts,
                    err
                );

                if connection_attempts >= max_connection_attempts {
                    tracing::error!("Max connections attempts reached");
                    exit(1);
                }

                connection_attempts += 1;

                // Wait before retrying
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
        };
    };

    let app = App::new(rx, startup_options, native_options, connection);
    app.run()?;

    Ok(())
}
