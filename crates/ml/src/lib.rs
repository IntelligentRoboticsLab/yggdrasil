//! This module provides the functionality necessary to run machine learning (ML)
//! models in the framework.

mod backend;
mod commands_ext;
mod element;
mod error;
pub mod util;

use bevy::prelude::*;

use backend::{MlCore, ModelExecutor};
use element::Parameters;

#[allow(missing_docs)]
pub mod prelude {
    pub use crate::backend::ModelExecutor;
    pub use crate::commands_ext::MlTaskCommandsExt;
    pub use crate::error::Error;
    pub use crate::util;
    pub use crate::{MlModel, MlModelResourceExt, MlPlugin};
}

/// Conveniency type representing an n-dimensional array.
pub type MlArray<E> = ndarray::ArrayD<E>;

/// Plugin offering a high level API for ML inference,
/// using the [OpenVINO](https://docs.openvino.ai/2023.3/home.html) runtime.
pub struct MlPlugin;

impl Plugin for MlPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.insert_resource(
            MlCore::new().expect("failed to initialize `MlCore` using the provided configuration!"),
        );
    }
}

/// A machine learning model.
///
/// A whole range of data types is supported,
/// e.g. f32, f64, u8, i32, etc.
///
/// ## Example
/// ```
/// use ml::prelude::*;
///
/// /// The Mixtral8x7b MoE model.
/// struct Mixtral8x7b;
///
/// impl MlModel for Mixtral8x7b {
///     // In this case, the model takes two inputs
///     type InputShape = (MlArray<u8>, MlArray<u8>);
///
///     // And produces a single output
///     type OutputShape = MlArray<u8>;
///
///     // This is the path to the model's ONNX file
///     const ONNX_PATH: &'static str = "deploy/models/mixtral8x7b.onnx";
/// }
/// ```
pub trait MlModel: Send + Sync + 'static {
    /// The model input shape.
    type Inputs: Parameters;

    /// The model output shape.
    type Outputs: Parameters;

    /// Path to the model's ONNX file.
    const ONNX_PATH: &'static str;
}

pub trait MlModelResourceExt {
    /// Adds a [`ModelExecutor`] to the app that can run the given model type.
    ///
    /// # Panics
    ///
    /// - If the MlCore does not exist, this function will panic.
    /// - If the model executor cannot be created, this function will panic.
    fn init_ml_model<M>(&mut self) -> &mut Self
    where
        Self: Sized,
        M: MlModel + Send + Sync + 'static;
}

impl MlModelResourceExt for App {
    fn init_ml_model<M>(&mut self) -> &mut Self
    where
        Self: Sized,
        M: MlModel + Send + Sync + 'static,
    {
        let mut ml_core = self
            .world_mut()
            .get_resource_mut::<MlCore>()
            .expect("the `MlCore` resource does not exist. Did you forget to add the `MlPlugin`?");

        let model_executor = ModelExecutor::<M>::new(&mut ml_core)
            .unwrap_or_else(|_| panic!("failed to create model executor for `{}`", M::ONNX_PATH));

        self.insert_resource(model_executor)
    }
}
