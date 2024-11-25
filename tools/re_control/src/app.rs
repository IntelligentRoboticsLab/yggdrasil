use std::net::Ipv4Addr;

use miette::{IntoDiagnostic, Result};
use re_control_comms::viewer::ControlViewer;
use re_viewer::StartupOptions;

use crate::control::Control;

// This is used for analytics, if the `analytics` feature is on in `Cargo.toml`
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

    pub async fn run(self) -> Result<()> {
        // let handle = self.viewer.run().await;

        let app_env = re_viewer::AppEnvironment::Custom(APP_ENV.to_string());

        // Listen for TCP connections from Rerun's logging SDKs.
        // There are other ways of "feeding" the viewer though - all you need is a `re_smart_channel::Receiver`.
        let rx = re_sdk_comms::serve(
            &Ipv4Addr::UNSPECIFIED.to_string(),
            re_sdk_comms::DEFAULT_SERVER_PORT,
            Default::default(),
        )
        .into_diagnostic()?;

        re_viewer::run_native_app(
            Box::new(move |cc| {
                let mut app = re_viewer::App::new(
                    re_viewer::build_info(),
                    &app_env,
                    self.startup_options,
                    cc.egui_ctx.clone(),
                    cc.storage,
                );
                app.add_receiver(rx);
                Box::new(Control::new(app, self.viewer))
            }),
            None,
        )
        .into_diagnostic()?;

        Ok(())
    }
}
