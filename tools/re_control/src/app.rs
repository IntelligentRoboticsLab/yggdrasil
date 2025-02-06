use std::net::Ipv4Addr;

use miette::{IntoDiagnostic, Result};
use re_viewer::StartupOptions;
use rerun::external::re_sdk_comms;

use crate::re_control_view::ControlView;

// Rerun can collect analytics if the `analytics` feature is enabled in
// `Cargo.toml`. This variable is used for the rerun analytics
const APP_ENV: &str = "Control Wrapper";

pub struct App {
    startup_options: StartupOptions,
}

impl App {
    pub fn new(startup_options: StartupOptions) -> Self {
        App { startup_options }
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

        // main_thread_token: MainThreadToken,
        // build_info: re_build_info::BuildInfo,
        // app_env: &crate::AppEnvironment,
        // startup_options: StartupOptions,
        // creation_context: &eframe::CreationContext<'_>,

        re_viewer::run_native_app(
            main_thread_token,
            Box::new(move |cc| {
                let mut app = re_viewer::App::new(
                    main_thread_token,
                    re_viewer::build_info(),
                    &app_env,
                    self.startup_options,
                    cc,
                );
                app.add_receiver(rx);

                // Register the custom view class
                app.add_view_class::<ControlView>().unwrap();

                Box::new(app)
            }),
            None,
        )
        .into_diagnostic()?;

        Ok(())
    }
}
