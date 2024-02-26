//! This is wip on what the API for the ML module could look like
//! it will probably be very similary to the `compute` example for the task
//! module in tyr/examples/compute.rs

use crate::prelude::*;

use self::backend::MLBackend;

mod backend;

pub trait Model {
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


#[startup_system]
fn initialize(storage: &mut Storage) -> Result<()> {
    let mut core = backend::load_core();

    let model = MLBackend::<ExampleModel>::new(&mut core);
    // storage.add_resource(Resource::new(model))?;
    Ok(())
}

#[system]
pub fn call_model(_model: &mut ComputeTask<ExampleModel>) -> Result<()> {
    // Do preprocessing here
    // let random_input = Vec::<u8>::new();

    // match model.run_inference(random_input) {
    //     Ok(_) => Ok(()),
    //     Err(Error::AlreadyActive) => Ok(()),
    // }

    Ok(())
}

#[system]
pub fn process_model_output(_model: &mut ComputeTask<ExampleModel>) -> Result<()> {
    // let random_input = Vec::<u8>::new();
    // model.call(random_input);

    // match model.call(random_input) {
    //     Ok(_) => Ok(()),
    //     Err(Error::AlreadyActive) => Ok(()),
    // }
    //
    // if let Some(output) = model.poll() else {
    //     return Ok(());
    // }
    
    // have output heredo whatever
    Ok(())
}

pub struct ExampleModule;

impl Module for ExampleModule {
    fn initialize(self, app: App) -> Result<App> {
        Ok(app
            .add_startup_system(initialize)?
            .add_system(process_model_output)
            .add_system(call_model)
        )
    }
}
