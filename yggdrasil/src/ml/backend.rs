//! Implementation of ML methods using an OpenVINO backend.

use super::{
    data_type::{Elem, InputElem, Output},
    Error, MlModel, Result,
};
use std::{marker::PhantomData, sync::Mutex};

/// Wrapper around [`openvino::Core`], i.e. the OpenVINO engine.
/// It's used for creating and using ML models.
pub struct MlCore(Mutex<openvino::Core>);

impl MlCore {
    pub fn new() -> Result<Self> {
        Ok(Self(Mutex::new(openvino::Core::new(None)?)))
    }
}

/// A ML model.
pub struct ModelExecutor<M: MlModel> {
    /// Model executor.
    exec: Mutex<openvino::ExecutableNetwork>,

    // descriptions of in- and output layer tensors
    input_descr: TensorDescr,
    output_descr: TensorDescr,
    _marker: PhantomData<M>,
}

impl<M: MlModel> ModelExecutor<M> {
    /// ## Error
    /// Fails if:
    /// * the model cannot be loaded.
    /// * an inference request cannot be created, which
    /// is needed to load relevant model settings.
    pub fn new(core: &mut MlCore) -> Result<Self> {
        let core = core.0.get_mut().unwrap();

        // load model
        let model = core
            .read_network_from_file(M::ONNX_PATH, "AUTO")
            .map_err(|e| Error::LoadModel {
                path: M::ONNX_PATH,
                source: e,
            })?;
        let mut exec = Mutex::new(
            core.load_network(&model, "CPU")
                .map_err(Error::LoadExecutableNetwork)?,
        );

        // unwrap is allowed because the model is guaranteed
        //  to have at least a single in- and output tensor
        let input_name = model.get_input_name(0).unwrap();
        let output_name = model.get_output_name(0).unwrap();

        // only through an inference request can we access
        //  in- and output tensor descriptions
        let mut infer = exec
            .get_mut()
            .unwrap()
            .create_infer_request()
            .map_err(Error::StartInference)?;

        let input_descr = TensorDescr {
            cfg: infer.get_blob(&input_name).unwrap().tensor_desc().unwrap(),
            name: input_name,
        };
        let output_descr = TensorDescr {
            cfg: infer.get_blob(&output_name).unwrap().tensor_desc().unwrap(),
            name: output_name,
        };

        // check if `M: MlModel` and loaded model in- and output types are compatible
        if !M::InputType::is_compatible(input_descr.cfg.precision()) {
            Err(Error::InputType {
                path: M::ONNX_PATH,
                expected: std::any::type_name::<M::InputType>().into(),
                imported: input_descr.cfg.precision(),
            })
        } else if !M::OutputType::is_compatible(output_descr.cfg.precision()) {
            Err(Error::OutputType {
                path: M::ONNX_PATH,
                expected: std::any::type_name::<M::OutputType>().into(),
                imported: input_descr.cfg.precision(),
            })
        } else {
            Ok(Self {
                exec,
                input_descr,
                output_descr,
                _marker: PhantomData,
            })
        }
    }

    /// Requests to run inference.
    pub fn request_infer(&mut self, input: &[M::InputType]) -> Result<InferRequest<M>> {
        let exec = self.exec.get_mut().unwrap();

        InferRequest::new(
            exec.create_infer_request().map_err(Error::StartInference)?,
            input,
            &self.input_descr,
            self.output_descr.clone(),
        )
    }
}

pub struct InferRequest<M: MlModel> {
    request: openvino::InferRequest,
    /// Output layer tensor description.
    output_descr: TensorDescr,
    // note `fn(M)` as opposed to just `M`, such that
    //  `Self` implements Send, even though `M` does not
    _marker: PhantomData<fn(M)>,
}

impl<M: MlModel> InferRequest<M> {
    fn new(
        mut request: openvino::InferRequest,
        input: &[M::InputType],
        input_descr: &TensorDescr,
        output_descr: TensorDescr,
    ) -> Result<Self> {
        // format input data
        let blob = openvino::Blob::new(
            &openvino::TensorDesc::new(
                input_descr.cfg.layout(),
                input_descr.cfg.dims(),
                input_descr.cfg.precision(),
            ),
            M::InputType::view_slice_bytes(input),
        )
        .map_err(Error::InferenceInput)?;

        // set input data
        request
            .set_blob(&input_descr.name, &blob)
            .map_err(Error::InferenceInput)?;

        Ok(Self {
            request,
            output_descr,
            _marker: PhantomData,
        })
    }

    /// Runs inference.
    pub fn infer(mut self) -> Result<Self> {
        self.request.infer().map_err(Error::RunInference)?;
        Ok(self)
    }

    pub fn get_output<O>(mut self) -> O
    where
        O: Output<M::OutputType>,
    {
        // the tensor called `output_name` is guaranteed to exist
        let blob = self.request.get_blob(&self.output_descr.name).unwrap();

        // we know the output tensor data type is compatible with `M::OutputType`
        //  due to the check in `MlBackend::new`, meaning it's safe
        //  to cast to this type
        let data = unsafe { blob.buffer_as_type::<M::OutputType>().unwrap() };

        O::from_slice(data, self.output_descr.cfg.dims())
    }
}

/// Description of a tensor.
struct TensorDescr {
    name: String,
    cfg: openvino::TensorDesc,
}

impl Clone for TensorDescr {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            cfg: openvino::TensorDesc::new(
                self.cfg.layout(),
                self.cfg.dims(),
                self.cfg.precision(),
            ),
        }
    }
}
