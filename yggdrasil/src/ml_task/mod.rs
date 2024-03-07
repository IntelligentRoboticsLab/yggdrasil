mod backend;
pub mod data_type;

use self::{
    backend::{MlBackend, MlInferRequest},
    data_type::{Elem, InputElem, Output},
};
use crate::ml_task::backend::MlCore;
use miette::Diagnostic;
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
pub trait MlModel: Send + 'static {
    /// Type of model input elements.
    type InputType: InputElem;
    /// Type of model output elements.
    type OutputType: Elem;

    /// Path to the model parameters.
    const ONNX_PATH: &'static str;
}

/// Machine learning (ML) task that runs ML models in a [`ComputeTask`].
pub struct MlTask<M: MlModel> {
    model: MlBackend<M>,
    task: ComputeTask<Result<MlInferRequest<M>>>,
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
    /// * The input failed to yield a byte view.
    pub fn try_start_infer(&mut self, input: &[M::InputType]) -> Result<()> {
        let req = self.model.request_infer(input)?;

        self.task.try_spawn(move || req.infer())?;
        Ok(())
    }

    /// Returns the output if available, or else [`None`].
    /// Once returned the same output cannot be polled again,
    /// so make sure to store the result or run inference again.
    /// ## Errors
    /// Returns `Some(`[`Error::RunInference`]`)` if an error occurred during
    /// inference.
    /// ## Return Collection
    /// The output is stored in the collection type provided by the user through
    /// the generic `O`, which can be anything that implements [`Output`].
    /// ## Example Usage
    /// ```
    /// use yggdrasil::ml_task::{MlTask, MlModel, data_type::MlArray};
    ///
    /// struct ResNet18;
    ///
    /// impl MlModel for ResNet18 {
    ///     type InputType = f32;
    ///     type OutputType = f32;
    ///     const ONNX_PATH: &'static str = "secret-folder/resnet18.onnx";
    /// }
    ///
    /// fn poll_the_thing(task: &mut MlTask<ResNet18>) -> MlArray<f32> {
    ///     // store the model output in an n-dimensional array
    ///     let output = task.poll::<MlArray<f32>>();
    ///     // we're sure there are no issues (never do this)
    ///     return output.unwrap().unwrap();
    /// }
    /// ```
    pub fn poll<O>(&mut self) -> Option<Result<O>>
    where
        O: Output<M::OutputType>,
    {
        let infer_result = self.task.poll()?;

        match infer_result {
            Ok(infer) => Some(Ok(infer.get_output())),
            Err(e) => Some(Err(e)),
        }
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

    #[error(
        "`MlModel` input type ({expected}) is incompatible with imported model input type\
            ({imported:?}) from `{path}`"
    )]
    InputType {
        path: &'static str,
        expected: String,
        imported: openvino::Precision,
    },

    #[error(
        "`MlModel` output type ({expected}) is incompatible with imported model output type\
            ({imported:?}) from `{path}`"
    )]
    OutputType {
        path: &'static str,
        expected: String,
        imported: openvino::Precision,
    },

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
