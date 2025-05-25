use std::net::{Ipv4Addr, SocketAddr, TcpStream};

use miette::{IntoDiagnostic, Result};
use rerun::external::{
    re_grpc_server, re_log,
    re_viewer::{self, AppEnvironment, AsyncRuntimeHandle, MainThreadToken, StartupOptions},
};

use crate::{control_view::ControlView, game_controller_view::GameControllerView};

// Rerun can collect analytics if the `analytics` feature is enabled in
// `Cargo.toml`. This variable is used for the rerun analytics
const APP_ENV: &str = "yggdrasil_rerun";

pub struct App {
    startup_options: StartupOptions,
}

impl App {
    pub fn new(startup_options: StartupOptions) -> Self {
        App { startup_options }
    }

    /// Check whether another server is running, if that's the case we should not spawn another instance.
    pub fn is_another_server_running(server_addr: &SocketAddr) -> bool {
        TcpStream::connect_timeout(&server_addr, std::time::Duration::from_secs(1)).is_ok()
    }

    pub async fn run(self, main_thread_token: MainThreadToken) -> Result<()> {
        let server_addr = SocketAddr::new(Ipv4Addr::UNSPECIFIED.into(), rerun::DEFAULT_SERVER_PORT);
        if Self::is_another_server_running(&server_addr) {
            re_log::info!(
                %server_addr,
                "A process is already listening at this address. Assuming it's a Rerun Viewer."
            );

            return Ok(());
        }

        // Listen for gRPC connections from Rerun's logging SDKs.
        // There are other ways of "feeding" the viewer though - all you need is a `re_smart_channel::Receiver`.
        let (rx_log, rx_table) = re_grpc_server::spawn_with_recv(
            SocketAddr::new(Ipv4Addr::UNSPECIFIED.into(), rerun::DEFAULT_SERVER_PORT),
            self.startup_options.memory_limit,
            re_grpc_server::shutdown::never(),
        );

        let app_env = AppEnvironment::Custom(APP_ENV.to_string());

        re_viewer::run_native_app(
            main_thread_token,
            Box::new(move |cc| {
                let mut app = re_viewer::App::new(
                    main_thread_token,
                    re_viewer::build_info(),
                    &app_env,
                    self.startup_options,
                    cc,
                    AsyncRuntimeHandle::from_current_tokio_runtime_or_wasmbindgen()
                        .expect("failed to obtain tokio runtime handle!"),
                );

                app.add_log_receiver(rx_log);
                app.add_table_receiver(rx_table);

                // Register the custom view classes
                app.add_view_class::<ControlView>().unwrap();
                app.add_view_class::<GameControllerView>().unwrap();

                Box::new(app)
            }),
            None,
        )
        .into_diagnostic()?;

        Ok(())
    }
}
