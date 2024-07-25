use miette::{IntoDiagnostic, Result};
use re_smart_channel::Receiver;
use re_viewer::{
    external::eframe::{self, NativeOptions},
    StartupOptions,
};
use rerun::log::LogMsg;

use crate::{connection::TcpConnection, seidr::Seidr};

// This is used for analytics, if the `analytics` feature is on in `Cargo.toml`
const APP_ENV: &str = "My Wrapper";

const WINDOW_TITLE: &str = "Seidr";

pub struct App {
    rx: Receiver<LogMsg>,
    startup_options: StartupOptions,
    native_options: NativeOptions,
    robot_connection: TcpConnection,
}

impl App {
    pub fn new(
        rx: Receiver<LogMsg>,
        startup_options: StartupOptions,
        native_options: NativeOptions,
        robot_connection: TcpConnection,
    ) -> Self {
        App {
            rx,
            startup_options,
            native_options,
            robot_connection,
        }
    }

    pub fn run(self) -> Result<()> {
        eframe::run_native(
            WINDOW_TITLE,
            self.native_options,
            Box::new(move |cc| {
                let _re_ui = re_viewer::customize_eframe_and_setup_renderer(cc);

                let mut rerun_app = re_viewer::App::new(
                    re_viewer::build_info(),
                    &re_viewer::AppEnvironment::Custom(APP_ENV.to_string()),
                    self.startup_options,
                    cc.egui_ctx.clone(),
                    cc.storage,
                );
                rerun_app.add_receiver(self.rx);

                let mut seidr = Seidr::new(rerun_app, self.robot_connection);
                seidr.listen_for_robot_responses();
                Ok(Box::new(seidr))
            }),
        )
        .into_diagnostic()?;

        Ok(())
    }
}
