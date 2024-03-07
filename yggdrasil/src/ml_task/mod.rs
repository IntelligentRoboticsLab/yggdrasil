pub mod backend;

use self::backend::{MlBackend, MlOutput};
use crate::ml_task::backend::MlCore;
use miette::{Diagnostic, IntoDiagnostic};
use thiserror::Error;
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
    /// Path to the model parameters.
    const ONNX_PATH: &'static str;
    /// Dimensions of the input.
    const INPUT_SHAPE: &'static [usize];
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
    pub fn try_start_infer(&mut self, input: &[u8]) -> Result<()> {
        let req = self.model.request_infer(input)?;

        self.task.try_spawn(move || req.infer())?;
        Ok(())
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
    fn add_ml_task<M>(self) -> miette::Result<Self>
    where
        Self: Sized,
        M: MlModel + Send + Sync + 'static;
}

impl MlTaskResource for App {
    fn add_ml_task<M>(self) -> miette::Result<Self>
    where
        Self: Sized,
        M: MlModel + Send + Sync + 'static,
    {
        fn add_ml_task<M: MlModel + Send + Sync + 'static>(
            storage: &mut Storage,
            mut core: ResMut<MlCore>,
            dispatcher: Res<ComputeDispatcher>,
        ) -> miette::Result<()> {
            storage.add_resource(Resource::new(
                MlTask::<M>::new(&mut core, dispatcher.clone()).into_diagnostic()?,
            ))
        }

        self.add_startup_system(add_ml_task::<M>)
    }
}

/// Instantiates the necessary resources to work with ML models.
pub struct MlModule;

impl Module for MlModule {
    fn initialize(self, app: App) -> miette::Result<App> {
        app.add_resource(Resource::new(MlCore::new()?))
    }
}

#[derive(Debug, Error, Diagnostic)]
pub enum Error {
    #[error("Failed to load OpenVINO core engine")]
    LoadCore(#[from] openvino::SetupError),

    #[error("Failed to load model from `{path}`")]
    LoadModel {
        path: &'static str,
        #[source]
        source: openvino::InferenceError,
    },

    #[error("Failed to create executable network")]
    LoadExecutableNetwork(#[source] openvino::InferenceError),

    #[error("Failed to start inference")]
    StartInference(#[source] openvino::InferenceError),

    #[error("Inference input does not meet model requirements")]
    InferenceInput(#[source] openvino::InferenceError),

    #[error("Failed to run inference")]
    RunInference(#[source] openvino::InferenceError),

    #[error(transparent)]
    Tyr(#[from] tyr::tasks::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
