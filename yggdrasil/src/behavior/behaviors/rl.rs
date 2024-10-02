use crate::core::ml::MlModel;

pub struct PolicyModel;

impl MlModel for PolicyModel {
    type InputType = f32;

    type OutputType = f32;

    const ONNX_PATH: &'static str = "models/rl_policy.onnx";
}
