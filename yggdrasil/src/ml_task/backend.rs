//! Implementation of ML methods using an OpenVINO backend.

use super::{Error, MlModel, Result};
use std::{marker::PhantomData, sync::Mutex};

pub type MlOutput = Result<Vec<u8>>;

/// Wrapper around [`openvino::Core`], i.e. the OpenVINO engine.
/// It's used for creating and using ML models.
pub struct MlCore(Mutex<openvino::Core>);

impl MlCore {
    pub fn new() -> Result<Self> {
        Ok(Self(Mutex::new(openvino::Core::new(None)?)))
    }
}

/// An ML model.
pub struct MlBackend<M: MlModel> {
    /// Model executor.
    exec: Mutex<openvino::ExecutableNetwork>,

    // names of in- and output layer
    input_name: String,
    output_name: String,
    _marker: PhantomData<M>,
}

impl<M: MlModel> MlBackend<M> {
    /// ## Error
    /// Fails if the model cannot be loaded.
    pub fn new(core: &mut MlCore) -> Result<Self> {
        let core = core.0.get_mut().unwrap();

        // load model
        let model = core
            .read_network_from_file(M::ONNX_PATH, "AUTO")
            .map_err(|e| Error::LoadModel {
                path: M::ONNX_PATH,
                source: e,
            })?;
        let exec = Mutex::new(
            core.load_network(&model, "CPU")
                .map_err(Error::LoadExecutableNetwork)?,
        );

        // unwrap is allowed because the model is guaranteed
        //  to have at least a single in- and output tensor
        let input_name = model.get_input_name(0).unwrap();
        let output_name = model.get_output_name(0).unwrap();

        Ok(Self {
            exec,
            input_name,
            output_name,
            _marker: PhantomData,
        })
    }

    /// Requests to run inference.
    pub fn request_infer(&mut self, input: &[u8]) -> Result<MlInferRequest> {
        let exec = self.exec.get_mut().unwrap();

        MlInferRequest::new::<M>(
            exec.create_infer_request().map_err(Error::StartInference)?,
            input,
            &self.input_name,
            self.output_name.clone(),
        )
    }
}

pub struct MlInferRequest {
    request: openvino::InferRequest,
    /// Name of output layer.
    output_name: String,
}

impl MlInferRequest {
    fn new<M: MlModel>(
        mut request: openvino::InferRequest,
        input: &[u8],
        input_name: &str,
        output_name: String,
    ) -> Result<Self> {
        // format input data
        let blob = openvino::Blob::new(
            &openvino::TensorDesc::new(
                openvino::Layout::NCHW,
                M::INPUT_SHAPE,
                openvino::Precision::FP32,
            ),
            input,
        )
        .map_err(Error::InferenceInput)?;

        // set input data
        request
            .set_blob(input_name, &blob)
            .map_err(Error::InferenceInput)?;

        Ok(Self {
            request,
            output_name,
        })
    }

    /// Runs inference.
    pub fn infer(mut self) -> MlOutput {
        self.request.infer().map_err(Error::RunInference)?;

        Ok(self
            .request
            .get_blob(&self.output_name)
            .unwrap() // fine, because the tensor called output_name is guaranteed to exist
            .buffer_mut()
            .unwrap()
            .to_vec())
    }
}
