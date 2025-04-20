//! Implementation of ML methods using an `OpenVINO` backend.

use crate::element::Parameters;

// use super::{
//     element::Parameters,
//     error::{Error, Result},
//     MlModel,
// };
use super::MlModel;

use bevy::prelude::*;
use miette::{IntoDiagnostic, Result};
use ort::{
    io_binding::IoBinding,
    memory::Allocator,
    session::{builder::GraphOptimizationLevel, Input, Output, Session},
    tensor::TensorElementType,
    value::{Tensor, ValueType},
};
use std::marker::PhantomData;

// TODO(Rick): Setup environment as part of the ml-ort plugin

/// Wrapper around a compiled ML model that implements Send + Sync.
#[derive(Deref, DerefMut)]
pub struct CompiledModel(Session);

// TODO(Rick): No idea if this is needed. Comes from ml
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
    // TODO(Rick): I think this can be removed since the Session contains Inputs and Outpus
    // Descriptions of in- and output layer tensors
    // input_descriptions: Arc<[TensorDescription]>,
    // output_descriptions: Arc<[TensorDescription]>,
    // binding: Arc<Mutex<IoBinding>>,
    _marker: PhantomData<M>,
}

impl<M: MlModel> ModelExecutor<M> {
    /// # Errors
    ///
    /// Fails if:
    /// - The model cannot be loaded.
    /// - An inference request cannot be created, which
    ///   is needed to load relevant model settings.
    pub fn new() -> Result<Self> {
        let compiled_model = {
            // weights_path parameter is unused for ONNX
            // TODO(Rick): This can be removed. But reuse the Error::LoadModel
            // let model =
            //     core.read_model_from_file(M::ONNX_PATH, "")
            //         .map_err(|e| Error::LoadModel {
            //             path: M::ONNX_PATH,
            //             source: e,
            //         })?;

            let model = Session::builder()
                .into_diagnostic()?
                .with_optimization_level(GraphOptimizationLevel::Level3)
                .into_diagnostic()?
                .commit_from_file(M::ONNX_PATH)
                .into_diagnostic()?;

            CompiledModel(model)
        };

        Ok(Self {
            compiled_model,
            _marker: PhantomData,
        })
    }

    fn inputs(&self) -> std::slice::Iter<Input> {
        self.compiled_model.inputs.iter()
    }

    fn outputs(&self) -> std::slice::Iter<Output> {
        self.compiled_model.outputs.iter()
    }

    // TODO(Rick): Probably only support tensors. So give error and/or message if input
    // is not a tensor.
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
        let mut binding = self.compiled_model.create_binding().into_diagnostic()?;

        let input_descriptions = self.compiled_model.inputs.iter();
        // for (description, input) in
        //     itertools::izip!(&self.compiled_model.inputs, inputs.iter_data())
        // {
        //     // Check if the input is of the correct type (The ValueType in ort) and if
        //     // the number of elements equal to the expected number of inputs
        //     let ValueType::Tensor { dimensions, .. } = &description.input_type else {
        //         // TODO(Rick): Send tracing log that input is not checked against
        //         // expected input
        //         println!("[WARN] Input was no tensor, therefore is not checked");
        //         return Err(todo!());
        //     };

        //     let expected: i64 = dimensions.iter().product();
        //     // if !input.is_tensor() {
        //     //     return Err(todo!("Give error that input should be tensor"))
        //     // }
        //     // Unwrap since at this point we already checked if it is a tensor
        //     // let actual = input.shape().unwrap().iter().product();

        //     let actual = inputs.num_elements() as i64;
        //     assert_eq!(expected, actual, "Input has the wrong amount of elements!");
        //     let tensor = Tensor::from_array((dimensions, inputs)).unwrap();

        //     binding
        //         .bind_input(description.name.clone(), &input)
        //         .into_diagnostic()?;
        // }
        for (description, input) in itertools::izip!(
            &self.compiled_model.inputs,
            inputs.iter_data(input_descriptions)
        ) {
            binding
                .bind_input(description.name.clone(), &input)
                .into_diagnostic()?;
        }

        let allocator = Allocator::default();
        for description in &self.compiled_model.outputs {
            binding
                .bind_output_to_device(description.name.clone(), &allocator.memory_info())
                .into_diagnostic()?;
        }

        let tensor_descriptions = self
            .compiled_model
            .outputs
            .iter()
            .map(|output| TensorDescription::from_output(output))
            .collect();

        Ok(InferRequest {
            binding,
            tensor_descriptions,
            _marker: PhantomData,
        })
    }

    /// Iterator over the input tensors.
    pub fn input_descriptions(&self) -> std::slice::Iter<Input> {
        self.inputs()
    }

    /// Iterator over the output tensors.
    pub fn output_descriptions(&self) -> std::slice::Iter<Output> {
        self.outputs()
    }
}

/// Model inference request.
///
/// This contains the openvino inference request, as well as the
/// descriptions of the output tensors.
pub struct InferRequest<M: MlModel> {
    binding: IoBinding,
    tensor_descriptions: Vec<TensorDescription>,
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
    pub fn run(&mut self) -> M::Outputs {
        let mut outputs = self.binding.run().into_diagnostic().unwrap();

        // TODO(Rick): Put check like in the `crate:ml` for output name and number of
        // expected elements
        // TODO(Rick): Just a horible name
        let mut tensors = vec![];
        for description in &self.tensor_descriptions {
            let value = outputs.remove(description.name.clone()).unwrap();
            tensors.push(value);
        }

        M::Outputs::from_tensors(tensors.iter())
    }

    /*
    /// Fetches the output tensor.
    ///
    /// # Panics
    ///
    /// - If the output tensor does not have the expected number of elements.
    /// - If the output tensor is not found, which should never happen.
    #[must_use]
    pub fn fetch_output(self) -> Value {
        let output = Tensor::<f32>::new(&Allocator::default(), [1, 32]).unwrap();

        let iter = self.output_descriptions.iter().map(|description| {
            let output = self
                .binding
                .bind_output(description.name, output)
                .expect("Cannot find output tensor!");

            // TODO(Rick): Probably should put this back
            // assert_eq!(
            //     output.get_size().unwrap(),
            //     description.num_elements(),
            //     "Output does not have the expected number of elements!"
            // );

            output
        });

        output.into()
    }
    */
}

/*
/// Wrapper around [`openvino::Shape`] that implements Send + Sync.
#[derive(Deref)]
struct Shape(openvino::Shape);

/// # Safety
///
/// This struct is Send + Sync because there is no internal mutability.
unsafe impl Send for Shape {}
unsafe impl Sync for Shape {}
*/

/// Description of a tensor parameter.
pub struct TensorDescription {
    name: String,
    shape: Vec<i64>,
    dtype: TensorElementType,
}

impl TensorDescription {
    // fn new(node: Node) -> Result<Self> {
    //     Ok(Self {
    //         name: node.get_name()?,
    //         shape: Shape(node.get_shape()?),
    //         dtype: node.get_element_type()?,
    //     })
    // }

    fn from_output(output: &Output) -> Self {
        Self {
            name: output.name.clone(),
            shape: output
                .output_type
                .tensor_dimensions()
                .expect("invalid tensor")
                .to_vec(),
            dtype: output.output_type.tensor_type().expect("invalid tensor"),
        }
    }

    /// Name of the tensor.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Dimensions of the tensor.
    pub fn dims(&self) -> &Vec<i64> {
        &self.shape
    }

    /// Number of elements in the tensor.
    pub fn num_elements(&self) -> usize {
        self.shape.iter().product::<i64>() as usize
    }

    /// Data type of the tensor.
    pub fn dtype(&self) -> TensorElementType {
        self.dtype
    }

    // Creates an empty tensor following the description.
    // pub fn to_empty_tensor(&self) -> Tensor {
    //     Tensor::::new(self.dtype, &self.shape).expect("Failed to create tensor from description")
    // }
}
