use std::net::Ipv4Addr;

use miette::{IntoDiagnostic, Result};
use re_control_comms::viewer::ControlViewer;
use re_viewer::{
    external::{eframe, egui},
    StartupOptions,
};

use crate::control::Control;

// Rerun can collect analytics if the `analytics` feature is enabled in
// `Cargo.toml`. This variable is used for the rerun analytics
const APP_ENV: &str = "Control Wrapper";

pub struct App {
    startup_options: StartupOptions,
    viewer: ControlViewer,
}

impl App {
    pub fn new(startup_options: StartupOptions, viewer: ControlViewer) -> Self {
        App {
            startup_options,
            viewer,
        }
    }

    pub async fn run(self, main_thread_token: re_viewer::MainThreadToken) -> Result<()> {
        let app_env = re_viewer::AppEnvironment::Custom(APP_ENV.to_string());

        // Listen for TCP connections from Rerun's logging SDKs.
        // There are other ways of "feeding" the viewer though - all you need is a `re_smart_channel::Receiver`.
        let rx = re_sdk_comms::serve(
            &Ipv4Addr::UNSPECIFIED.to_string(),
            re_sdk_comms::DEFAULT_SERVER_PORT,
            Default::default(),
        )
        .into_diagnostic()?;
        let native_options = eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default().with_app_id("yggdrasil_control"),
            ..re_viewer::native::eframe_options(None)
        };

        eframe::run_native(
            "Rerun",
            native_options,
            Box::new(move |cc| {
                re_viewer::customize_eframe_and_setup_renderer(cc)?;

                let mut app = re_viewer::App::new(
                    main_thread_token,
                    re_viewer::build_info(),
                    &app_env,
                    self.startup_options,
                    cc.egui_ctx.clone(),
                    cc.storage,
                );
                app.add_receiver(rx);
                Ok(Box::new(Control::new(app, self.viewer)))
            }),
        )
        .into_diagnostic()?;

        Ok(())
    }
}
