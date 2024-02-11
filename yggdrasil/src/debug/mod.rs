use miette::IntoDiagnostic;
use rerun::RecordingStream;
use std::time::Instant;

use crate::prelude::*;

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
fn init_rerun(storage: &mut Storage, ad: &AsyncDispatcher) -> Result<()> {
    let handle = ad.handle().clone();
    let _guard = handle.enter();
    let rec = rerun::RecordingStreamBuilder::new("example_nao")
        .serve(
            "0.0.0.0",
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
