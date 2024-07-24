//! This example shows how to wrap the Rerun Viewer in your own GUI.
use app::App;
use connection::TcpConnection;
use miette::{IntoDiagnostic, Result};
use re_viewer::external::{eframe, egui, re_log, re_memory};
use std::net::{Ipv4Addr, SocketAddrV4};

mod app;
mod connection;

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

    let mut startup_options = re_viewer::StartupOptions::default();
    // Limit memory to 1.5 GB
    startup_options.memory_limit = re_memory::MemoryLimit::from_bytes(1500000000);

    // Connect with robot 24
    // println!("Trying to connect to robot...");
    // let socket_addr = SocketAddrV4::new(Ipv4Addr::new(10, 1, 8, 24), 40001);
    let socket_addr = SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 40001);
    let connection = TcpConnection::try_from_ip(socket_addr).await?;

    let app = App::new(rx, startup_options, native_options, connection);

    app.run()?;

    Ok(())
}

// fn handle_serialized_rerun_message(serialized_msg: SerializedRerunMessage, config_data: Arc<Mutex<String>>) -> Result<()> {
//     println!("Message: {:?}", serialized_msg.decode_response()?);
//     match serialized_msg.decode_response()? {
//         RerunResponse::RequestConfigResponse(data) => {
//             *config_data.lock().unwrap() = data.to_string();
//         },
//         _ => {}
//     }

//     Ok(())
// }
