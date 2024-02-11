use miette::IntoDiagnostic;
use rerun::RecordingStream;
use std::time::Instant;

use crate::{nao::RobotInfo, prelude::*};

pub struct DebugModule;

impl Module for DebugModule {
    fn initialize(self, app: App) -> miette::Result<tyr::prelude::App> {
        Ok(app
            .add_startup_system(init_rerun)?
            .add_system(run_debug.after(crate::nao::write_hardware_info)))
    }
}

struct RerunStartTime(Instant);

#[startup_system]
fn init_rerun(storage: &mut Storage, ad: &AsyncDispatcher, robot_info: &RobotInfo) -> Result<()> {
    // To spawn a recording stream in serve mode, the tokio runtime needs to be in scope.
    let handle = ad.handle().clone();
    let _guard = handle.enter();

    // Manually set the server address to the robot's IP address, instead of 0.0.0.0
    // to ensure the rerun server prints the correct connection URL on startup
    let server_address = format!("10.0.8.{}", robot_info.robot_id);
    let rec = rerun::RecordingStreamBuilder::new("example_nao")
        .serve(
            &server_address,
            Default::default(),
            Default::default(),
            rerun::MemoryLimit::from_fraction_of_total(0.05),
            false,
        )
        .into_diagnostic()?;

    // Recording stream is a essentially an Arc, so we can freely clone it
    storage.add_resource(Resource::new(rec.clone()))?;
    storage.add_resource(Resource::new(RerunStartTime(Instant::now())))
}

#[system]
fn run_debug(rec: &RecordingStream, start_time: &RerunStartTime) -> Result<()> {
    rec.set_time_seconds("control_loop", start_time.0.elapsed().as_secs_f64());
    Ok(())
}
