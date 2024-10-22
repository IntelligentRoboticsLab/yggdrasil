//! See [`Error`].

use miette::Diagnostic;
use thiserror::Error;

/// Error types for this crate.
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
        "`MlModel` input type ({expected:?}) is incompatible with imported model input type\
            ({imported:?}) from `{path}`"
    )]
    InputType {
        path: &'static str,
        expected: openvino::Precision,
        imported: openvino::Precision,
    },

    #[error(
        "`MlModel` output type ({expected:?}) is incompatible with imported model output type\
            ({imported:?}) from `{path}`"
    )]
    OutputType {
        path: &'static str,
        expected: openvino::Precision,
        imported: openvino::Precision,
    },

    #[error("`MlModel` does not contain an input layer with index {0}!")]
    MissingInputLayer(usize),

    #[error("`MlModel` does not contain an output layer with index {0}!")]
    MissingOutputLayer(usize),

    #[error("Failed to start inference")]
    StartInference(#[source] openvino::InferenceError),

    #[error("Number of inputs in the model ({expected}) does not match the number of inputs provided ({actual})")]
    InputCountMismatch { expected: usize, actual: usize },

    #[error(
        "Inference input is of size {actual}, while the model expects an input of size {expected}"
    )]
    InferenceInputSize { expected: usize, actual: usize },

    #[error("Failed to run inference")]
    RunInference(#[source] openvino::InferenceError),

    #[error("Failed to create inference input tensor for model: {1}")]
    CreateInputTensor(#[source] openvino::InferenceError, &'static str),

    #[error("Failed to set blob for tensor `{1}` ({2})")]
    SetBlob(#[source] openvino::InferenceError, String, &'static str),

    #[error("OpenVINO threw an unexpected error")]
    UnexpectedOpenVino(#[from] openvino::InferenceError),
}

/// Type alias for [`Result<T, Error>`].
pub type Result<T> = std::result::Result<T, Error>;
