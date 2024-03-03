//! This is wip on what the API for the ML module could look like
//! it will probably be very similary to the `compute` example for the task
//! module in tyr/examples/compute.rs

// TODO: use evil-hack branch

use crate::prelude::*;

use self::backend::MLTask;

mod backend;

pub trait Model: 'static {
    /// Returns a path to the weights of the model.
    const ONNX: &'static str;
    const INPUT_SHAPE: [usize; 4];
}

// API usage below:
pub struct ExampleModel;

// First step would be just implenting by hand
impl Model for ExampleModel {
    const ONNX: &'static str = "weights.onnx";
    const INPUT_SHAPE: [usize; 4] = [0, 0, 0, 0];
}

// Later we could add a macro to generate the implementation:
// #[model(onnx = "/path/to/onnx"), input_shape=(480, 640)]
// pub struct ExampleModel;

#[system]
pub fn call_model(task: &mut MLTask<ExampleModel>) -> Result<()> {
    // do preprocessing here
    let random_input = Vec::<u8>::new();

    // try to start to run inference, if it fails it's already
    //  running inference
    let _ = task.try_infer(&random_input);

    // call `task.poll` to process the result
    Ok(())
}

#[system]
pub fn poll_model(task: &mut MLTask<ExampleModel>) -> Result<()> {
    if Some(res) = task.poll() {
        // Do some thing with the result
    }

    Ok(())
}

pub struct ExampleModule;

impl Module for ExampleModule {
    fn initialize(self, app: App) -> Result<App> {
        Ok(app
            .add_startup_system(initialize)?
            .add_task::<MLTask<ExampleModel>>()?
            .add_system(call_model)
            .add_system(poll_model))
    }
}
