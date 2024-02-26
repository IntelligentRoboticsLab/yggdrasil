use std::marker::PhantomData;
use openvino::{Blob, CNNNetwork, Core, ExecutableNetwork, Layout, Precision, TensorDesc};
use super::Model;

pub struct MLBackend<M: Model> {
    model: CNNNetwork,
    exec: ExecutableNetwork,
    _marker: PhantomData<M>
}

impl<M: Model> MLBackend<M> {
    // TODO: error handling
    pub fn new(core: &mut Core) -> Self {
        let model = core.read_network_from_file(M::ONNX, "AUTO").expect("Failed to load model data!");
        let exec = core.load_network(&model, "CPU").expect("Failed to load executable network!");

        Self {
            model, exec, _marker: PhantomData
        }
    }

    // TODO: error handling
    pub fn run_inference(&mut self, input: &[u8]) -> Vec<u8> {
        // names of in- and output layer
        let input_name = self.model.get_input_name(0).unwrap();
        let output_name = self.model.get_output_name(0).unwrap();

        let mut infer_request = self.exec.create_infer_request()
            .expect("Could not create infer request!");
        
        // set input data
        let blob = Blob::new(
            &TensorDesc::new(Layout::NCHW, &M::INPUT_SHAPE, Precision::FP32),
            input
        ).unwrap();
        infer_request.set_blob(&input_name, &blob).unwrap();

        // run inference
        infer_request.infer().expect("Failed inference!");
        return infer_request.get_blob(&output_name).unwrap()
            .buffer_mut().unwrap()
            // TODO: should we avoid this copy??
            .to_vec();
    }
}

// TODO: error handling
pub fn load_core() -> Core {
    Core::new(None).expect("Could not load OpenVINO core")
}