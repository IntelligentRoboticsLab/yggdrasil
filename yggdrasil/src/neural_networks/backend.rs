// TODO: move this into tyr.
use super::Model;
use openvino::{
    Blob, CNNNetwork, Core, ExecutableNetwork, InferRequest, Layout, Precision, TensorDesc,
};
use std::{marker::PhantomData, sync::Mutex};
use tyr::tasks::{
    compute::{ComputeDispatcher, ComputeTask},
    task::Pollable,
};

type MLInput = [u8];
type MLOutput = Vec<u8>;

/// A task designed to run machine learning models as a [`ComputeTask`].
pub struct MLTask<M: Model> {
    model: MLModel<M>,
    task: ComputeTask<MLOutput>,
}

impl<M: Model> MLTask<M> {
    pub fn new(dispatcher: ComputeDispatcher) -> Self {
        // TODO: Can we not do this at every new()?
        let mut core = backend::load_core();

        Self {
            model: MLModel::new(core),
            task: ComputeTask::new(dispatcher),
        }
    }

    /// Tries to run the model inference as a compute task.
    /// ## Errors
    /// Returns an [`tyr::tasks::Error::AlreadyActive`] if the task is active already.
    pub fn try_infer(&mut self, input: &MLInput) -> Result<(), tyr::tasks::Error> {
        let mut req = self.model.request_infer(input);

        self.task.try_spawn(move || req.infer())
    }

    pub fn poll(&mut self) -> Option<MLOutput> {
        self.task.poll()
    }
}

/// A machine learning model.
struct MLModel<M: Model> {
    model: CNNNetwork,
    exec: Mutex<ExecutableNetwork>,

    // names of in- and output layer
    input_name: String,
    output_name: String,
    _marker: PhantomData<M>,
}

impl<M: Model> MLModel<M> {
    // TODO: error handling
    fn new(core: &mut Core) -> Self {
        let model = core
            .read_network_from_file(M::ONNX, "AUTO")
            .expect("Failed to load model data!");
        let exec = Mutex::new(
            core.load_network(&model, "CPU")
                .expect("Failed to load executable network!"),
        );

        let input_name = model.get_input_name(0).unwrap();
        let output_name = model.get_output_name(0).unwrap();

        Self {
            model,
            exec,
            input_name,
            output_name,
            _marker: PhantomData,
        }
    }

    // TODO: error handling
    /// Request to run an inference.
    fn request_infer(&mut self, input: &[u8]) -> MLInferRequest<M> {
        MLInferRequest::new(
            self.exec.lock().unwrap().create_infer_request().unwrap(),
            input,
            &self.input_name,
            self.output_name.clone(),
        )
    }
}

struct MLInferRequest<M: Model> {
    req: InferRequest,
    /// Name of output layer.
    output_name: String,
    // TODO: Explain
    _marker: PhantomData<fn(M)>,
}

impl<M: Model> MLInferRequest<M> {
    // TODO: validate input size
    // Add add_ml_task, such that this may return an Result<>.
    fn new(mut req: InferRequest, input: &[u8], input_name: &str, output_name: String) -> Self {
        // set input data // TODO: explain
        let blob = Blob::new(
            &TensorDesc::new(Layout::NCHW, &M::INPUT_SHAPE, Precision::FP32),
            input,
        )
        .unwrap();
        req.set_blob(input_name, &blob).unwrap();

        Self {
            req,
            output_name,
            _marker: PhantomData,
        }
    }

    // TODO:
    // - error handling
    /// Run inference.
    fn infer(mut self) -> Vec<u8> {
        // run inference
        self.req.infer().expect("Failed inference!");

        self.req
            .get_blob(&self.output_name)
            .unwrap()
            .buffer_mut()
            .unwrap()
            // TODO: should we avoid this copy??
            .to_vec()
    }
}

// TODO: error handling
pub fn load_core() -> Core {
    Core::new(None).expect("Could not load OpenVINO core")
}
