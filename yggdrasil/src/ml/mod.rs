//! This module provides the functionality necessary to run machine learing (ML)
//! models in the framework.

pub mod backend;
pub mod data_type;
pub mod util;

use self::{
    backend::{InferRequest, ModelExecutor},
    data_type::{Elem, InputElem, Output},
};
use crate::ml::backend::MlCore;
use miette::Diagnostic;
use thiserror::Error;
use tyr::{
    prelude::{startup_system, AsyncDispatcher, AsyncTask},
    tasks::task::Pollable,
    App, Module, Resource, Storage,
};

/// A machine learning model.
///
/// A whole range of data types is supported,
/// e.g. f32, f64, u8, i32, etc.
///
/// ## Example
/// ```
/// use yggdrasil::ml::MlModel;
///
/// /// The Mixtral8x7b MoE model.
/// struct Mixtral8x7b;
///
/// impl MlModel for Mixtral8x7b {
///     // these are the in- and output data types
///     type InputType = u8;
///     type OutputType = u8;
///     // this is the path to the model's ONNX file
///     const ONNX_PATH: &'static str = "deploy/models/mixtral8x7b.onnx";
/// }
/// ```
pub trait MlModel: 'static {
    /// Type of model input elements.
    type InputType: InputElem;
    /// Type of model output elements.
    type OutputType: Elem;

    /// Path to the model parameters.
    const ONNX_PATH: &'static str;
}

/// Machine learning (ML) task that runs a [`MlModel`] in an [`AsyncTask`].
///
/// ## Example Usage
/// Add a ML task to the app as follows:
/// ```
/// use tyr::App;
/// use yggdrasil::ml::{MlModel, MlTaskResource, MlModule};
///
/// fn build_app<Model: MlModel + Send + Sync>() -> miette::Result<()> {
///     let app = App::new()
///         .add_module(MlModule)?
///         .add_ml_task::<Model>()?;
///     Ok(())
/// }
/// ```
pub struct MlTask<M: MlModel> {
    model: ModelExecutor<M>,
    task: AsyncTask<Result<InferRequest<M>>>,
}

impl<M: MlModel> MlTask<M> {
    pub fn new(core: &mut MlCore, dispatcher: AsyncDispatcher) -> Result<Self> {
        Ok(Self {
            model: ModelExecutor::new(core)?,
            task: AsyncTask::new(dispatcher),
        })
    }

    /// Attempts to run the model as a compute task on
    /// a separate thread.
    ///
    /// ## Errors
    /// Fails if:
    /// * The input size is incorrect.
    /// * The task was not yet finished from a previous call.
    /// * Inference could not be started for some internal reason.
    pub fn try_start_infer(&mut self, input: &[M::InputType]) -> Result<()> {
        let infer_req = self.model.request_infer(input)?;

        self.task.try_spawn_blocking(|| infer_req.run())?;
        Ok(())
    }

    pub fn cancel(&mut self) {
        self.task.try_cancel();
    }

    /// Returns the output if available, or else [`None`].
    /// Once returned the same output cannot be polled again,
    /// so make sure to store the result or run inference again.
    ///
    /// ## Errors
    /// Returns `Some(`[`Error::RunInference`]`)` if an error occurred during
    /// inference.
    ///
    /// ## Returns
    /// The output is stored in the collection type provided by the user through
    /// the generic `O`, which can be anything that implements [`Output`].
    ///
    /// ## Example Usage
    /// ```
    /// use yggdrasil::ml::{MlTask, MlModel, data_type::MlArray, Result};
    ///
    /// struct ResNet18;
    ///
    /// impl MlModel for ResNet18 {
    ///     type InputType = f32;
    ///     type OutputType = f32;
    ///     const ONNX_PATH: &'static str = "secret-folder/resnet18.onnx";
    /// }
    ///
    /// fn poll_the_thing(task: &mut MlTask<ResNet18>) -> Option<Result<MlArray<f32>>> {
    ///     // store the model output in an n-dimensional array
    ///     task.poll::<MlArray<f32>>()
    /// }
    /// ```
    pub fn poll<O>(&mut self) -> Option<Result<O>>
    where
        O: Output<M::OutputType>,
    {
        let infer_result = self.task.poll()?;

        match infer_result {
            Ok(infer_req) => Some(infer_req.fetch_output()),
            Err(e) => Some(Err(e)),
        }
    }

    /// Returns whether the task is currently active.
    pub fn active(&self) -> bool {
        self.task.active()
    }

    pub fn model(&self) -> &ModelExecutor<M> {
        &self.model
    }
}

pub trait MlTaskResource {
    /// Adds a [`MlTask`] to the app that can run the given model type.
    ///
    /// ## Errors
    /// Fails if no [`MlCore`] is in storage, which is supplied by [`MlModule`].
    fn add_ml_task<M>(self) -> miette::Result<Self>
    where
        Self: Sized,
        M: MlModel + Send + Sync + 'static;
}

impl MlTaskResource for App {
    fn add_ml_task<M>(self) -> miette::Result<Self>
    where
        Self: Sized,
        M: MlModel + Send + Sync,
    {
        #[startup_system]
        fn add_ml_task<M: MlModel + Send + Sync>(
            storage: &mut Storage,
            core: &mut MlCore,
            dispatcher: &AsyncDispatcher,
        ) -> crate::Result<()> {
            let task = MlTask::<M>::new(core, dispatcher.clone())?;

            storage.add_resource(Resource::new(task))
        }

        self.add_startup_system(add_ml_task::<M>)
    }
}

/// A module offering a high level API for ML inference,
/// using the [OpenVINO](https://docs.openvino.ai/2023.3/home.html) runtime.
///
/// This module provides the following resources to the application:
/// - [`MlCore`]
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

    #[error(
        "Inference input is of size {actual}, while the model expects an input of size {expected}"
    )]
    InferenceInputSize { expected: usize, actual: usize },

    #[error("Failed to run inference")]
    RunInference(#[source] openvino::InferenceError),

    #[error("OpenVINO threw an unexpected error")]
    UnexpectedOpenVino(#[source] openvino::InferenceError),

    #[error(transparent)]
    Tyr(#[from] tyr::tasks::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
