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
    input_descriptions: Vec<TensorDescr>,
    output_descriptions: Vec<TensorDescr>,
    _marker: PhantomData<M>,
}

impl<M: MlModel> ModelExecutor<M> {
    /// ## Error
    /// Fails if:
    /// * The model cannot be loaded.
    /// * An inference request cannot be created, which
    ///   is needed to load relevant model settings.
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

        let mut infer = exec
            .get_mut()
            .unwrap()
            .create_infer_request()
            .map_err(Error::StartInference)?;

        // the model is guaranteed to have at least a single in- and output tensor
        let num_inputs = model.get_inputs_len()?;
        let mut input_descrs = Vec::with_capacity(num_inputs);
        for i in 0..num_inputs {
            let input_name = model.get_input_name(i)?;
            let tensor = TensorDescr {
                cfg: infer.get_blob(&input_name)?.tensor_desc()?,
                name: input_name,
            };

            if !M::InputType::is_compatible(tensor.cfg.precision()) {
                return Err(Error::InputType {
                    path: M::ONNX_PATH,
                    expected: std::any::type_name::<M::InputType>().into(),
                    imported: tensor.cfg.precision(),
                });
            }
            input_descrs.push(tensor);
        }

        let num_outputs = model.get_outputs_len()?;
        let mut output_descrs = Vec::with_capacity(num_outputs);
        for i in 0..model.get_outputs_len()? {
            let output_name = model.get_output_name(i)?;
            let tensor = TensorDescr {
                cfg: infer.get_blob(&output_name)?.tensor_desc()?,
                name: output_name,
            };
            if !M::OutputType::is_compatible(tensor.cfg.precision()) {
                return Err(Error::OutputType {
                    path: M::ONNX_PATH,
                    expected: std::any::type_name::<M::InputType>().into(),
                    imported: tensor.cfg.precision(),
                });
            }
            output_descrs.push(tensor);
        }

        Ok(Self {
            exec,
            input_descriptions: input_descrs,
            output_descriptions: output_descrs,
            _marker: PhantomData,
        })
    }

    /// Requests to run inference.
    pub fn request_infer(&mut self, input: &[&[M::InputType]]) -> Result<InferRequest<M>> {
        let exec = self.exec.get_mut().unwrap();

        InferRequest::new(
            exec.create_infer_request().map_err(Error::StartInference)?,
            input,
            &self.input_descriptions,
            &self.output_descriptions,
        )
    }

    /// Description of the input tensor.
    pub fn input_description(&self, index: usize) -> Result<&TensorDescr> {
        self.input_descriptions
            .get(index)
            .ok_or(Error::MissingInputLayer(index))
    }

    /// Description of the output tensor.
    pub fn output_description(&self, index: usize) -> Result<&TensorDescr> {
        self.output_descriptions
            .get(index)
            .ok_or(Error::MissingOutputLayer(index))
    }
}

pub struct InferRequest<M: MlModel> {
    request: openvino::InferRequest,
    /// Output layer tensor description.
    output_descrs: Vec<TensorDescr>,
    // note `fn() -> M` as opposed to just `M`, such that
    //  `Self` implements Send, even though `M` does not
    _marker: PhantomData<fn() -> M>,
}

impl<M: MlModel> InferRequest<M> {
    fn new(
        mut request: openvino::InferRequest,
        inputs: &[&[M::InputType]],
        input_descrs: &[TensorDescr],
        output_descrs: &[TensorDescr],
    ) -> Result<Self> {
        for (description, input) in input_descrs.iter().zip(inputs) {
            let cfg = &description.cfg;

            // check if input is of correct size
            if input.len() != cfg.len() {
                return Err(Error::InferenceInputSize {
                    expected: cfg.dims()[0],
                    actual: input.len(),
                });
            }

            // load input data
            let blob = openvino::Blob::new(
                &openvino::TensorDesc::new(cfg.layout(), cfg.dims(), cfg.precision()),
                M::InputType::view_slice_bytes(input),
            )
            .map_err(Error::UnexpectedOpenVino)?;

            // set input data
            request
                .set_blob(&description.name, &blob)
                .map_err(Error::UnexpectedOpenVino)?;
        }

        Ok(Self {
            request,
            output_descrs: output_descrs.to_vec(),
            _marker: PhantomData,
        })
    }

    /// Runs inference.
    pub fn run(mut self) -> Result<Self> {
        self.request.infer().map_err(Error::RunInference)?;
        Ok(self)
    }

    pub fn fetch_output<O>(mut self) -> Result<Vec<O>>
    where
        O: Output<M::OutputType>,
    {
        // the tensor with the name `output_name` is guaranteed to exist
        let blobs = self
            .output_descrs
            .into_iter()
            .map(|x| {
                let blob = self.request.get_blob(&x.name).unwrap();

                // we know the output tensor data type is compatible with `M::OutputType`
                // due to the check in `ModelExecutor::new`, meaning it's safe
                // to cast to this type
                let data = unsafe { blob.buffer_as_type::<M::OutputType>() }.unwrap();

                O::from_slice(data, x.cfg.dims())
            })
            .collect();
        Ok(blobs)
    }
}

/// Description of a tensor.
pub struct TensorDescr {
    name: String,
    cfg: openvino::TensorDesc,
}

impl TensorDescr {
    /// Dimensions of the tensor.
    pub fn dims(&self) -> &[usize] {
        self.cfg.dims()
    }
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
