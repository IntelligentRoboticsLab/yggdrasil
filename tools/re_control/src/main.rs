use std::net::SocketAddrV4;

use clap::Parser;
use miette::{IntoDiagnostic, Result};
use re_control::{app::App, cli::Cli};
use re_viewer::external::{re_log, re_memory};

use re_control_comms::{protocol::CONTROL_PORT, viewer::ControlViewer};

const BYTES_IN_MB: i64 = 1_000_000;
const MEMORY_FRACTION_DEFAULT: f32 = 0.75;

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

    tracing::info!(
        "Starting rerun control and connection with {}",
        args.robot_ip
    );

    // Install handlers for panics and crashes that prints to stderr and send
    // them to Rerun analytics (if the `analytics` feature is on in `Cargo.toml`).
    re_crash_handler::install_crash_handlers(re_viewer::build_info());

    // Setting a memory limit of 75% or a limit defined via the cli arguments
    let memory_limit = if let Some(max_memory) = args.max_mem {
        re_memory::MemoryLimit::parse(&max_memory)
            .unwrap_or_else(|_| panic!("Failed to parse `{}` to a `MemoryLimit`", max_memory))
    } else {
        re_memory::MemoryLimit::from_fraction_of_total(MEMORY_FRACTION_DEFAULT)
    };

    tracing::info!(
        "Memory limit set to: {} MB",
        memory_limit.max_bytes.unwrap() / BYTES_IN_MB
    );

    let startup_options = re_viewer::StartupOptions {
        memory_limit,
        ..Default::default()
    };

    let socket_addr = SocketAddrV4::new(args.robot_ip, CONTROL_PORT);
    let viewer = ControlViewer::from_addr(socket_addr).into_diagnostic()?;

    let app = App::new(startup_options, viewer);
    app.run().await?;

    Ok(())
}
