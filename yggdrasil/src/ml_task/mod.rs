pub mod backend;

use self::backend::{MlBackend, MlOutput};
use crate::ml_task::backend::MlCore;
use miette::{Context, IntoDiagnostic, Result};
use tyr::{
    tasks::{
        compute::{ComputeDispatcher, ComputeTask},
        task::Pollable,
    },
    App, Module, Res, ResMut, Resource, Storage,
};

/// A ML model type. For now, only models
/// with 32 bit floating point precision work properly.
pub trait MlModel {
    /// Returns a path to the model parameters.
    fn onnx_path() -> &'static str;
    /// Returns the dimensions of the input.
    fn input_shape() -> Vec<usize>;
}

/// Machine learning (ML) task that runs ML models in a [`ComputeTask`].
pub struct MlTask<M: MlModel> {
    model: MlBackend<M>,
    task: ComputeTask<MlOutput>,
}

impl<M: MlModel> MlTask<M> {
    pub fn new(core: &mut MlCore, dispatcher: ComputeDispatcher) -> Result<Self> {
        Ok(Self {
            model: MlBackend::new(core)?,
            task: ComputeTask::new(dispatcher),
        })
    }

    /// Tries to run the model inference as a compute task on
    /// a separate thread.
    /// ## Errors
    /// Fails if:
    /// * The task was not yet finished from a previous call.
    /// * Inference could not be started, e.g. because the input size is incorrect.
    /// * Inference started, but failed at some point for some reason.
    pub fn try_start_infer(&mut self, input: &[u8]) -> Result<()> {
        let req = self.model.request_infer(input)?;

        self.task
            .try_spawn(move || req.infer())
            .into_diagnostic()
            .wrap_err("Failed ML inference")
    }

    /// Checks if the output is available and returns it.
    /// Once returned the same output cannot be polled again,
    /// so make sure to store the result or run inference again.
    pub fn poll(&mut self) -> Option<MlOutput> {
        self.task.poll()
    }
}

pub trait MlTaskResource {
    /// Adds a [`MlTask`] to the app that can run the given model type.
    /// ## Errors
    /// Fails if no [`MlCore`] is in storage. Add [`MlModule`] to the app to accomplish this.
    fn add_ml_task<M>(self) -> Result<Self>
    where
        Self: Sized,
        M: MlModel + Send + Sync + 'static;
}

impl MlTaskResource for App {
    fn add_ml_task<M>(self) -> Result<Self>
    where
        Self: Sized,
        M: MlModel + Send + Sync + 'static,
    {
        fn add_ml_task<M: MlModel + Send + Sync + 'static>(
            storage: &mut Storage,
            mut core: ResMut<MlCore>,
            dispatcher: Res<ComputeDispatcher>,
        ) -> Result<()> {
            storage.add_resource(Resource::new(MlTask::<M>::new(
                &mut core,
                dispatcher.clone(),
            )?))
        }

        self.add_startup_system(add_ml_task::<M>)
    }
}

/// Instantiates the necessary resources to work with ML models.
pub struct MlModule;

impl Module for MlModule {
    fn initialize(self, app: App) -> Result<App> {
        app.add_resource(Resource::new(MlCore::new()?))
    }
}
