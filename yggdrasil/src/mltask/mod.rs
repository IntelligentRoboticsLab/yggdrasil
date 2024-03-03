// TODO: move into tyr?

use miette::{Context, IntoDiagnostic, Result};
use tyr::{tasks::{compute::{ComputeDispatcher, ComputeTask}, task::Pollable}, App, Module, Res, ResMut, Resource, Storage};
use crate::mltask::backend::MLCore;
use self::backend::{MLBackend, MLOutput};

mod backend;

/// A ML model type. For now, only models
/// with 32 bit floating point precision work properly.
pub trait MLModel {
    /// Returns a path to the model parameters.
    fn onnx_path() -> &'static str;
    /// Returns the dimensions of the input.
    fn input_shape() -> Vec<usize>;
}

/// Machine learning (ML) task that runs ML models in a [`ComputeTask`].
pub struct MLTask<M: MLModel> {
    model: MLBackend<M>,
    task: ComputeTask<MLOutput>
}

impl<M: MLModel> MLTask<M> {
    pub fn new(core: &mut MLCore, dispatcher: ComputeDispatcher) -> Result<Self> {
        Ok(Self {
            model: MLBackend::new(core)?,
            task: ComputeTask::new(dispatcher)
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

        self.task.try_spawn(move || {
            req.infer()
        }).into_diagnostic().wrap_err("Failed ML inference")
    }

    /// Checks if the output is available and returns it.
    /// Once returned the same output cannot be polled again,
    /// so make sure to store the result or run inference again.
    pub fn poll(&mut self) -> Option<MLOutput> {
        self.task.poll()
    }
}

pub trait MLTaskResource {
    /// Adds a [`MLTask`] to the app that can run the given model type.
    fn add_ml_task<M>(self) -> Result<Self>
        where 
            Self: Sized,
            M: MLModel + Send + Sync + 'static;
}

impl MLTaskResource for App {
    fn add_ml_task<M>(self) -> Result<Self>
        where
            Self: Sized,
            M: MLModel + Send + Sync + 'static
    {
        fn add_ml_task<M: MLModel + Send + Sync + 'static>(
            storage: &mut Storage,
            dispatcher: Res<ComputeDispatcher>,
            mut core: ResMut<MLCore>
        ) -> Result<()>
        {
            storage.add_resource(
                Resource::new(MLTask::<M>::new(&mut core, dispatcher.clone()))
            )
        }
        
        self.add_startup_system(add_ml_task::<M>)
    }
}

/// Instantiates the necessary resources to load ML models.
pub struct MLModule;

impl Module for MLModule {
    fn initialize(self, app: App) -> Result<App> {
        Ok(app.add_resource(Resource::new(MLCore::new()))?)
    }
}