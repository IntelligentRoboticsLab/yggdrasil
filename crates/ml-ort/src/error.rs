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

    #[error("Failed to compile model")]
    CompileError(#[source] openvino::InferenceError),

    #[error(
        "`MlModel` input type ({expected:?}) is incompatible with imported model input type\
            ({imported:?}) from `{path}`"
    )]
    InputType {
        path: &'static str,
        expected: openvino::ElementType,
        imported: openvino::ElementType,
    },

    #[error(
        "`MlModel` output type ({expected:?}) is incompatible with imported model output type\
            ({imported:?}) from `{path}`"
    )]
    OutputType {
        path: &'static str,
        expected: openvino::ElementType,
        imported: openvino::ElementType,
    },

    #[error("Failed to start inference")]
    StartInference(#[source] openvino::InferenceError),

    #[error("Failed to run inference")]
    RunInference(#[source] openvino::InferenceError),

    #[error("OpenVINO threw an unexpected error")]
    UnexpectedOpenvino(#[from] openvino::InferenceError),
}

/// Type alias for [`Result<T, Error>`].
pub type Result<T> = std::result::Result<T, Error>;
