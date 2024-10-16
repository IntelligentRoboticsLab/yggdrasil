//! This module provides the functionality necessary to run machine learning (ML)
//! models in the framework.

mod backend;
mod commands_ext;
mod element_type;
mod error;
pub mod util;

use self::{
    backend::ModelExecutor,
    element_type::{input::ModelInput, output::ModelOutput, Elem},
};
use backend::MlCore;
use bevy::prelude::*;

#[allow(missing_docs)]
pub mod prelude {
    pub use crate::backend::{InferRequest, MlCore, ModelExecutor};
    pub use crate::commands_ext::MlTaskCommandsExt;
    pub use crate::element_type::input::{InputContainer, ModelInput};
    pub use crate::element_type::output::{ModelOutput, OutputContainer};
    pub use crate::element_type::{Elem, MlArray};
    pub use crate::error::Error;

    pub use crate::util as ml_util;

    pub use crate::{MlModel, MlModelResourceExt, MlPlugin};
}

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
///     // these are the in- and output data types
///     type InputElem = u8;
///     type OutputElem = u8;
///
///     // this is the shape of the model's input
///     type InputShape = (MlArray<u8>, MlArray<u8>);
///
///     // this is the shape of the model's output
///     type OutputShape = (MlArray<u8>,);
///
///     // this is the path to the model's ONNX file
///     const ONNX_PATH: &'static str = "deploy/models/mixtral8x7b.onnx";
/// }
/// ```
pub trait MlModel: Send + Sync + 'static {
    /// The input element type of the model.
    type InputElem: Elem;
    /// The output element type of the model.
    type OutputElem: Elem;

    /// The shape of the model input.
    type InputShape: ModelInput<Self::InputElem>;

    /// The shape of the model output.
    type OutputShape: ModelOutput<Self::OutputElem>;

    /// Path to the model parameters.
    const ONNX_PATH: &'static str;
}

pub trait MlModelResourceExt {
    /// Adds a [`ModelExecutor`] to the app that can run the given model type.
    ///
    /// # Panics
    ///
    /// - If the [`MlCore`] resource does not exist, this function will panic.
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
