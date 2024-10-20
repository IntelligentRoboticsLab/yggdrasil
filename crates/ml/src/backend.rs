//! Implementation of ML methods using an `OpenVINO` backend.
use super::{
    element::Parameters,
    error::{Error, Result},
    MlModel,
};
use bevy::prelude::*;
use std::{marker::PhantomData, sync::Mutex};

/// Wrapper around [`openvino::Core`], i.e. the `OpenVINO` engine.
/// It's used for creating and using ML models.
#[derive(Resource)]
pub struct MlCore(Mutex<openvino::Core>);

impl MlCore {
    /// Create a new `OpenVINO` core.
    ///
    /// # Errors
    ///
    /// Fails if the core cannot be created.
    pub fn new() -> Result<Self> {
        Ok(Self(Mutex::new(openvino::Core::new(None)?)))
    }
}

/// A ML model.
#[derive(Resource)]
pub struct ModelExecutor<M: MlModel> {
    /// Model executor.
    exec: Mutex<openvino::ExecutableNetwork>,

    // descriptions of in- and output layer tensors
    input_descriptions: Vec<TensorDescr>,
    output_descriptions: Vec<TensorDescr>,
    _marker: PhantomData<M>,
}

impl<M: MlModel> ModelExecutor<M> {
    /// # Errors
    ///
    /// Fails if:
    /// - The model cannot be loaded.
    /// - An inference request cannot be created, which
    ///   is needed to load relevant model settings.
    ///
    /// # Panics
    ///
    /// If no mutable reference to the model executor can be obtained this function panics.
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

        for (i, dtype) in M::Inputs::data_types().enumerate() {
            let input_name = model.get_input_name(i)?;
            let tensor = TensorDescr {
                cfg: infer.get_blob(&input_name)?.tensor_desc()?,
                name: input_name.clone(),
            };

            let onnx_dtype = tensor.cfg.precision();
            if dtype != onnx_dtype {
                return Err(Error::InputType {
                    path: M::ONNX_PATH,
                    expected: dtype,
                    imported: onnx_dtype,
                });
            }

            input_descrs.push(tensor);
        }

        let num_outputs = model.get_outputs_len()?;
        let mut output_descrs = Vec::with_capacity(num_outputs);
        for (i, dtype) in M::Outputs::data_types().enumerate() {
            let output_name = model.get_output_name(i)?;
            let tensor = TensorDescr {
                cfg: infer.get_blob(&output_name)?.tensor_desc()?,
                name: output_name.clone(),
            };

            let onnx_dtype = tensor.cfg.precision();
            if dtype != onnx_dtype {
                return Err(Error::OutputType {
                    path: M::ONNX_PATH,
                    expected: dtype,
                    imported: onnx_dtype,
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
    ///
    /// # Errors
    ///
    /// Fails if the inference request cannot be created.
    ///
    /// # Panics
    ///
    /// Panics if a mutable reference to the model executor cannot be obtained.
    pub fn request_infer(&mut self, input: &M::Inputs) -> Result<InferRequest<M>> {
        let exec = self.exec.get_mut().expect("Failed to lock model executor.");

        InferRequest::new(
            exec.create_infer_request().map_err(Error::StartInference)?,
            input.blobs(),
            &self.input_descriptions,
            &self.output_descriptions,
        )
    }

    /// Description of the input tensor.
    ///
    /// # Errors
    ///
    /// Fails if there is no input layer at the given index.
    pub fn input_description(&self, index: usize) -> Result<&TensorDescr> {
        self.input_descriptions
            .get(index)
            .ok_or(Error::MissingInputLayer(index))
    }

    /// Description of the output tensor.
    ///
    /// # Errors
    ///
    /// Fails if the output layer does not exist at the given index.
    pub fn output_description(&self, index: usize) -> Result<&TensorDescr> {
        self.output_descriptions
            .get(index)
            .ok_or(Error::MissingOutputLayer(index))
    }
}

/// Model inference request.
///
/// This contains the openvino inference request, as well as the
/// descriptions of the output tensors.
pub struct InferRequest<M: MlModel> {
    request: openvino::InferRequest,
    /// Output layer tensor description.
    output_descrs: Vec<TensorDescr>,
    // note `fn() -> M` as opposed to just `M`, such that
    // `Self` implements Send, even though `M` does not
    //
    // Also see [the relevant Nomicon chapter.](https://doc.rust-lang.org/nomicon/subtyping.html)
    _marker: PhantomData<fn() -> M>,
}

impl<M: MlModel> InferRequest<M> {
    fn new<'a>(
        mut request: openvino::InferRequest,
        inputs: impl Iterator<Item = &'a [u8]>,
        input_descrs: &[TensorDescr],
        output_descrs: &[TensorDescr],
    ) -> Result<Self> {
        if M::Inputs::len() != input_descrs.len() {
            return Err(Error::InputCountMismatch {
                expected: input_descrs.len(),
                actual: M::Inputs::len(),
            });
        }

        for (description, input, dtype_size) in
            itertools::izip!(input_descrs, inputs, M::Inputs::sizes_of())
        {
            let cfg = &description.cfg;

            let expected = cfg
                .dims()
                .iter()
                .copied()
                .reduce(|a, b| a * b)
                .expect("Failed to compute number of inputs!");

            // check if input is of correct size
            let input_len = input.len() / dtype_size;
            if input_len != expected {
                return Err(Error::InferenceInputSize {
                    expected,
                    actual: input.len(),
                });
            }

            // load input data
            let blob = openvino::Blob::new(
                &openvino::TensorDesc::new(cfg.layout(), cfg.dims(), cfg.precision()),
                input,
            )
            .map_err(|e| Error::CreateInputTensor(e, M::ONNX_PATH))?;

            // set input data
            request
                .set_blob(&description.name, &blob)
                .map_err(|e| Error::SetBlob(e, description.name.to_string(), M::ONNX_PATH))?;
        }

        Ok(Self {
            request,
            output_descrs: output_descrs.to_vec(),
            _marker: PhantomData,
        })
    }

    /// Runs inference.
    ///
    /// # Errors
    ///
    /// Returns an error if the inference fails for any reason.
    /// See [`Error`] for more details.
    pub fn run(mut self) -> Result<Self> {
        self.request.infer().map_err(Error::RunInference)?;
        Ok(self)
    }

    /// Fetches the output tensor.
    ///
    /// # Panics
    ///
    /// Panics if the output tensor is not found, which should never happen.
    #[must_use]
    pub fn fetch_output(mut self) -> M::Outputs {
        let iter = self.output_descrs.iter().map(|descr| {
            let blob = self.request.get_blob(&descr.name).unwrap();
            let dims = descr.dims();

            assert_eq!(
                blob.len().unwrap(),
                descr.num_elements(),
                "Blob does not have the expected size!"
            );

            (dims, blob)
        });

        // # Safety:
        //
        // If this fails I blame the openvino-rs developers
        unsafe { M::Outputs::from_dims_and_blobs(iter) }
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

    /// The total number of elements in the tensor.
    pub fn num_elements(&self) -> usize {
        self.cfg
            .dims()
            .iter()
            .copied()
            .reduce(|a, b| a * b)
            .unwrap_or_default()
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
