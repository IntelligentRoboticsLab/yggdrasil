use std::env;

use build_manager::version::Version;
use clap::Parser;
use miette::Result;
use re_control::{app::App, cli::Cli, RerunControl};
use re_viewer::external::{re_log, re_memory};

const BYTES_IN_GB: f32 = 1_000_000_000.0;
const MEMORY_FRACTION_DEFAULT: f32 = 0.75;

// By using `re_memory::AccountingAllocator` Rerun can keep track of exactly how much memory it is using,
// and prune the data store when it goes above a certain limit.
// By using `mimalloc` we get faster allocations.
#[global_allocator]
static GLOBAL: re_memory::AccountingAllocator<mimalloc::MiMalloc> =
    re_memory::AccountingAllocator::new(mimalloc::MiMalloc);

#[tokio::main]
async fn main() -> Result<()> {
    RerunControl::check_current_version();

    let main_thread_token = re_viewer::MainThreadToken::i_promise_i_am_on_the_main_thread();
    re_log::setup_logging();

    let args = Cli::parse();

    re_crash_handler::install_crash_handlers(re_viewer::build_info());

    // Setting a memory limit of 75% or a limit defined via the cli arguments
    let memory_limit = if let Some(max_memory) = args.max_mem {
        re_memory::MemoryLimit::parse(&max_memory)
            .unwrap_or_else(|_| panic!("Failed to parse `{}` to a `MemoryLimit`", max_memory))
    } else {
        re_memory::MemoryLimit::from_fraction_of_total(MEMORY_FRACTION_DEFAULT)
    };

    // Communicate the memory limit
    re_log::info!(
        "Memory limit set to: {:.2} GB",
        memory_limit.max_bytes.unwrap() as f32 / BYTES_IN_GB
    );
    // Setting startup options for the rerun viewer
    let startup_options = re_viewer::StartupOptions {
        memory_limit,
        ..Default::default()
    };

    // Storing the robot ip address (if specified) to be used in the `re_control_view`
    if let Some(robot_ip) = args.robot_ip {
        env::set_var("ROBOT_ADDR", robot_ip.to_string());
    }

    let app = App::new(startup_options);
    app.run(main_thread_token).await?;

    Ok(())
}
