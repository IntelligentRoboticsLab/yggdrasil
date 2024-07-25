//! This example shows how to wrap the Rerun Viewer in your own GUI.
use app::App;
use connection::TcpConnection;
use miette::{IntoDiagnostic, Result};
use re_viewer::external::{eframe, egui, re_log, re_memory};
use std::{
    net::{Ipv4Addr, SocketAddrV4},
    time::Duration,
};
use yggdrasil::core::control::CONTROL_PORT;

mod app;
mod connection;
mod resource;
mod seidr;

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
        viewport: egui::ViewportBuilder::default().with_app_id("rerun_extend_viewer_ui_example"),
        ..re_viewer::native::eframe_options(None)
    };

    let startup_options = re_viewer::StartupOptions {
        // Limit memory to 1.5 GB
        memory_limit: re_memory::MemoryLimit::from_bytes(1500000000),
        ..Default::default()
    };

    // let socket_addr = SocketAddrV4::new(Ipv4Addr::new(10, 1, 8, 24), CONTROL_PORT);
    let socket_addr = SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), CONTROL_PORT);

    // Tries to make a connection to the robot address
    let connection = loop {
        match TcpConnection::try_from_ip(socket_addr).await {
            Ok(conn) => break conn,
            Err(err) => {
                eprintln!("Failed to connect: {}. Retrying...", err);
                // Optionally, wait before retrying
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        };
    };

    let app = App::new(rx, startup_options, native_options, connection);
    app.run()?;

    Ok(())
}
