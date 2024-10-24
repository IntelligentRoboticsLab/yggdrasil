//! Implementation of ML methods using an `OpenVINO` backend.
use super::{
    element::Parameters,
    error::{Error, Result},
    MlModel,
};
use bevy::prelude::*;
use openvino::{Node, RwPropertyKey, Tensor};
use std::{marker::PhantomData, sync::Arc};

/// Wrapper around [`openvino::Core`], i.e. the `OpenVINO` engine.
/// It's used for creating and using ML models.
#[derive(Resource, Deref, DerefMut)]
pub struct Core(openvino::Core);

/// # Safety
///
/// This struct is Sync because there is no internal mutability.
unsafe impl Sync for Core {}

impl Core {
    /// Create a new `OpenVINO` core.
    ///
    /// # Errors
    ///
    /// Fails if the core cannot be created.
    pub fn new() -> Result<Self> {
        let mut core = openvino::Core::new()?;

        // TODO: we should test cycle time without this limit
        core.set_property(
            &openvino::DeviceType::CPU,
            &RwPropertyKey::InferenceNumThreads,
            "1",
        )?;

        Ok(Self(core))
    }
}

/// Wrapper around a compiled ML model that implements Send + Sync.
#[derive(Deref, DerefMut)]
pub struct CompiledModel(openvino::CompiledModel);

/// # Safety
///
/// This struct is Sync because there is no internal mutability.
unsafe impl Sync for CompiledModel {}

/// A compiled ML model with the descriptions for its in-/outputs.
///
/// Used to run inference on the model.
#[derive(Resource)]
pub struct ModelExecutor<M: MlModel> {
    compiled_model: CompiledModel,
    // Descriptions of in- and output layer tensors
    input_descriptions: Arc<[TensorDescription]>,
    output_descriptions: Arc<[TensorDescription]>,
    _marker: PhantomData<M>,
}

impl<M: MlModel> ModelExecutor<M> {
    /// # Errors
    ///
    /// Fails if:
    /// - The model cannot be loaded.
    /// - An inference request cannot be created, which
    ///   is needed to load relevant model settings.
    pub fn new(core: &mut Core) -> Result<Self> {
        let compiled_model = {
            // weights_path parameter is unused for ONNX
            let model =
                core.read_model_from_file(M::ONNX_PATH, "")
                    .map_err(|e| Error::LoadModel {
                        path: M::ONNX_PATH,
                        source: e,
                    })?;

            CompiledModel(
                core.compile_model(&model, openvino::DeviceType::CPU)
                    .map_err(Error::CompileError)?,
            )
        };

        let input_descriptions = Self::get_input_descriptions(&compiled_model)?;
        let output_descriptions = Self::get_output_descriptions(&compiled_model)?;

        Ok(Self {
            compiled_model,
            input_descriptions,
            output_descriptions,
            _marker: PhantomData,
        })
    }

    fn get_input_descriptions(model: &CompiledModel) -> Result<Arc<[TensorDescription]>> {
        let num_inputs = model.get_input_size()?;
        assert_eq!(
            num_inputs,
            M::Inputs::len(),
            "Number of inputs does not match model file!"
        );

        let mut input_descrs = Vec::with_capacity(num_inputs);
        for (i, dtype) in M::Inputs::data_types().enumerate() {
            let node = model.get_input_by_index(i)?;
            let tensor = TensorDescription::new(node)?;

            if dtype != tensor.dtype {
                return Err(Error::InputType {
                    path: M::ONNX_PATH,
                    expected: dtype,
                    imported: tensor.dtype,
                });
            }

            input_descrs.push(tensor);
        }

        Ok(input_descrs.into())
    }

    fn get_output_descriptions(model: &CompiledModel) -> Result<Arc<[TensorDescription]>> {
        let num_outputs = model.get_output_size()?;
        assert_eq!(
            num_outputs,
            M::Outputs::len(),
            "Number of outputs does not match model file!"
        );

        let mut output_descrs = Vec::with_capacity(num_outputs);
        for (i, dtype) in M::Outputs::data_types().enumerate() {
            let node = model.get_output_by_index(i)?;
            let tensor = TensorDescription::new(node)?;

            if dtype != tensor.dtype {
                return Err(Error::OutputType {
                    path: M::ONNX_PATH,
                    expected: dtype,
                    imported: tensor.dtype,
                });
            }

            output_descrs.push(tensor);
        }

        Ok(output_descrs.into())
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
    pub fn request_infer(&mut self, inputs: &M::Inputs) -> Result<InferRequest<M>> {
        let mut request = self
            .compiled_model
            .create_infer_request()
            .map_err(Error::StartInference)?;

        for (description, input, dtype_size) in itertools::izip!(
            self.input_descriptions(),
            inputs.blobs(),
            M::Inputs::sizes_of()
        ) {
            // Check if input has the correct amount of elements
            let expected = description.num_elements();
            let actual = input.len() / dtype_size;
            assert_eq!(expected, actual, "Input has the wrong amount of elements!");

            let mut tensor = description.to_empty_tensor();
            {
                let data = tensor.get_raw_data_mut()?;
                data.copy_from_slice(input);
            }

            request.set_tensor(description.name(), &tensor)?;
        }

        let output_descriptions = self.output_descriptions.clone();

        Ok(InferRequest {
            request,
            output_descriptions,
            _marker: PhantomData,
        })
    }

    /// Iterator over the input tensors.
    pub fn input_descriptions(&self) -> std::slice::Iter<TensorDescription> {
        self.input_descriptions.iter()
    }

    /// Iterator over the output tensors.
    pub fn output_descriptions(&self) -> std::slice::Iter<TensorDescription> {
        self.output_descriptions.iter()
    }
}

/// Model inference request.
///
/// This contains the openvino inference request, as well as the
/// descriptions of the output tensors.
pub struct InferRequest<M: MlModel> {
    request: openvino::InferRequest,
    output_descriptions: Arc<[TensorDescription]>,
    // note `fn() -> M` as opposed to just `M`, such that
    // `Self` implements Send, even though `M` does not
    //
    // Also see [the relevant Nomicon chapter.](https://doc.rust-lang.org/nomicon/subtyping.html)
    _marker: PhantomData<fn() -> M>,
}

impl<M: MlModel> InferRequest<M> {
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
    /// - If the output tensor does not have the expected number of elements.
    /// - If the output tensor is not found, which should never happen.
    #[must_use]
    pub fn fetch_output(self) -> M::Outputs {
        let iter = self.output_descriptions.iter().map(|description| {
            let output = self
                .request
                .get_tensor(description.name())
                .expect("Cannot find output tensor!");

            assert_eq!(
                output.get_size().unwrap(),
                description.num_elements(),
                "Output does not have the expected number of elements!"
            );

            output
        });

        // # Safety:
        //
        // If this fails I blame the openvino-rs developers
        unsafe { M::Outputs::from_tensors(iter) }
    }
}

/// Wrapper around [`openvino::Shape`] that implements Send + Sync.
#[derive(Deref)]
struct Shape(openvino::Shape);

/// # Safety
///
/// This struct is Send + Sync because there is no internal mutability.
unsafe impl Send for Shape {}
unsafe impl Sync for Shape {}

/// Description of a tensor parameter.
pub struct TensorDescription {
    name: String,
    shape: Shape,
    dtype: openvino::ElementType,
}

impl TensorDescription {
    fn new(node: Node) -> Result<Self> {
        Ok(Self {
            name: node.get_name()?,
            shape: Shape(node.get_shape()?),
            dtype: node.get_element_type()?,
        })
    }

    /// Name of the tensor.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Dimensions of the tensor.
    pub fn dims(&self) -> &[i64] {
        self.shape.get_dimensions()
    }

    /// Number of elements in the tensor.
    pub fn num_elements(&self) -> usize {
        self.shape.get_dimensions().iter().product::<i64>() as usize
    }

    /// Data type of the tensor.
    pub fn dtype(&self) -> openvino::ElementType {
        self.dtype
    }

    /// Creates an empty tensor following the description.
    pub fn to_empty_tensor(&self) -> Tensor {
        Tensor::new(self.dtype, &self.shape).expect("Failed to create tensor from description")
    }
}
