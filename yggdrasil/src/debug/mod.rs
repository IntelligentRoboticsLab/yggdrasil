use std::f32::consts::TAU;

use rerun::external::glam;
use miette::IntoDiagnostic;

use crate::prelude::*;

pub struct DebugModule;

impl Module for DebugModule {
    fn initialize(self, app: App) -> miette::Result<tyr::prelude::App> {
        Ok(app.add_startup_system(init_rerun)?
            .add_system(run_debug.after(crate::nao::write_hardware_info)))
    }
}


#[derive(Debug)]
pub struct DebugMachine {
    rec_stream: rerun::RecordingStream,
    pub behaviour_origins: Vec<rerun::Position2D>,
    pub behaviour_directions: Vec<rerun::Vector2D>,
    pub behaviour_labels: Vec<String>,
    pub behaviour_color: Vec<rerun::Color>,
}

impl DebugMachine {
    fn new(rec: rerun::RecordingStream) -> DebugMachine {
        DebugMachine {
            rec_stream: rec,
            behaviour_origins: Vec::new(),
            behaviour_directions: Vec::new(),
            behaviour_labels:  Vec::new(),
            behaviour_color: Vec::new(),
        }
    }

    pub fn log_behavior(&mut self) {
        self.rec_stream.log("behaviour/transitions",
            &rerun::Arrows2D::from_vectors(self.behaviour_directions.clone())
            .with_radii([0.025])
            .with_origins(self.behaviour_origins.clone())
            .with_colors(self.behaviour_color.clone())
            .with_labels(self.behaviour_labels.clone()),
        );
    }
}

#[startup_system]
fn init_rerun(storage: &mut Storage, ad: &AsyncDispatcher) -> Result<()> {

    // let handle = storage.map_resource_ref(|ad: &AsyncDispatcher| ad.handle().clone())?;

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

    // rec.flush_blocking();
    storage.add_resource(Resource::new(DebugMachine::new(rec)))
}

#[system]
pub fn run_debug(
    panel: &mut DebugMachine
) -> Result<()> {
    //engine.current_behavior;
    // Check if the behaviour needs to be updated
    //  update the behaviour
    panel.log_behavior();
    Ok(())
}