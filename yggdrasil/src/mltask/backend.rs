use std::{marker::PhantomData, sync::Mutex};
use miette::{Context, IntoDiagnostic, Result};
use openvino::{Blob, InferRequest, Layout, Precision, TensorDesc};
use super::MLModel;

pub type MLOutput = Result<Vec<u8>>;

/// Wrapper around [`openvino::Core`], i.e. the OpenVINO engine.
/// It's used for creating and using ML models.
pub struct MLCore(Mutex<openvino::Core>);

impl MLCore {
    pub fn new() -> Result<Self> {
        Ok(Self(Mutex::new(
            openvino::Core::new(None)
                .into_diagnostic().wrap_err("Failed to initialize OpenVINO core engine")?
        )))
    }
}

/// A ML model.
pub struct MLBackend<M: MLModel> {
    /// Model executor.
    exec: Mutex<openvino::ExecutableNetwork>,

    // names of in- and output layer
    input_name: String,
    output_name: String,
    _marker: PhantomData<M>
}

impl<M: MLModel> MLBackend<M> {
    /// ## Error
    /// Fails if the model cannot be loaded.
    pub fn new(core: &mut MLCore) -> Result<Self> {
        let core = core.0.get_mut().unwrap();

        // load model
        let model = core.read_network_from_file(M::onnx_path(), "AUTO")
            .into_diagnostic().wrap_err("Failed to load ML .onnx file")?;
        let exec = Mutex::new(core.load_network(&model, "CPU")
            .into_diagnostic().wrap_err("Failed to load ML model on device")?);

        let input_name = model.get_input_name(0).unwrap();
        let output_name = model.get_output_name(0).unwrap();

        Ok(Self {
            exec, input_name, output_name, _marker: PhantomData
        })
    }

    /// Requests to run inference.
    pub fn request_infer(&mut self, input: &[u8]) -> Result<MLInferRequest> {
        let exec = self.exec.get_mut().unwrap();

        MLInferRequest::new::<M>(
            exec.create_infer_request().into_diagnostic().wrap_err("Failed to start ML inference")?,
            input,
            &self.input_name,
            self.output_name.clone()
        )
    }
}

pub struct MLInferRequest {
    req: InferRequest,
    /// Name of output layer.
    output_name: String
}

impl MLInferRequest {
    fn new<M: MLModel>(
        mut req: InferRequest, input: &[u8], input_name: &str, output_name: String
    ) -> Result<Self> {
        // format input data
        let blob = Blob::new(
            &TensorDesc::new(Layout::NCHW, &M::input_shape(), Precision::FP32),
            input
        ).into_diagnostic().wrap_err(
            "ML inference input does not meet model input requirements"
        )?;

        // set input data
        req.set_blob(input_name, &blob).unwrap();

        Ok(Self {
            req, output_name
        })
    }

    /// Runs inference.
    pub fn infer(mut self) -> MLOutput {      
        // run inference
        self.req.infer().into_diagnostic().wrap_err("Failed ML inference")?;

        Ok(self.req.get_blob(&self.output_name).unwrap()
            .buffer_mut().unwrap()
            .to_vec()
        )
    }
}