//! This module provides the functionality necessary to run machine learing (ML)
//! models in the framework.

mod backend;
mod commands_ext;
mod element_type;
mod error;
pub mod util;

use self::{
    backend::ModelExecutor,
    element_type::{Elem, InputElem},
};
use backend::MlCore;
use bevy::prelude::*;

#[allow(missing_docs)]
pub mod prelude {
    pub use crate::backend::{InferRequest, MlCore, ModelExecutor};
    pub use crate::commands_ext::MlTaskCommandsExt;
    pub use crate::element_type::{Elem, InputElem};
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
/// use yggdrasil::core::ml::MlModel;
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
pub trait MlModel: Send + Sync + 'static {
    /// Type of model input elements.
    type InputType: InputElem;
    /// Type of model output elements.
    type OutputType: Elem;

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
        M: MlModel + Send + Sync,
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
