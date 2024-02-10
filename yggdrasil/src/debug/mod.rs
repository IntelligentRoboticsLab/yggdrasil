use miette::IntoDiagnostic;

use crate::prelude::*;

pub struct DebugModule;

impl Module for DebugModule {
    fn initialize(self, app: App) -> miette::Result<tyr::prelude::App> {
        Ok(app
            .add_startup_system(init_rerun)?
            .add_system(run_debug.after(crate::nao::write_hardware_info)))
    }
}

#[derive(Debug)]
pub struct DebugMachine {
    rec_stream: rerun::RecordingStream,
}

impl DebugMachine {
    fn new(rec: rerun::RecordingStream) -> DebugMachine {
        DebugMachine { rec_stream: rec }
    }

    pub fn log_behavior(&mut self) {
        self.rec_stream.log(
            "behaviour/transitions",
            &rerun::Arrows2D::from_vectors([[1.0, 0.0], [0.0, -1.0], [-0.7, 0.7]])
                .with_radii([0.025])
                .with_origins([[0.25, 0.0], [0.25, 0.0], [-0.1, -0.1]])
                .with_colors([[255, 0, 0], [0, 255, 0], [127, 0, 255]])
                .with_labels(["right", "up", "left-down"]),
        );
    }
}

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

    storage.add_resource(Resource::new(DebugMachine::new(rec)))
}

#[system]
pub fn run_debug(panel: &mut DebugMachine) -> Result<()> {
    panel.log_behavior();
    Ok(())
}
