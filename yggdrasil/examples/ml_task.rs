//! This is an example of how to implement a ML model.
//! The code does not run, but a model can be implemented
//! with the same structure, provided model parameters
//! and logic for processing in- and output is written.

use miette::Result;
use tyr::{
    system,
    tasks::{TaskConfig, TaskModule},
    App, Resource,
};
use yggdrasil::ml_task::{data_type::MlArray, MlModel, MlModule, MlTask, MlTaskResource};

struct ResNet18;

// implement MLModel to make it compatible with the rest of the system
impl MlModel for ResNet18 {
    type InputType = f32;
    type OutputType = f32;

    const ONNX_PATH: &'static str = "secret-folder/resnet18.onnx";
}

struct Image(Option<MlArray<f32>>);

#[system]
fn process_image(
    // ML model (note the MLTask wrapper)
    model: &mut MlTask<ResNet18>,
    // model input
    image: &mut Image,
) -> Result<()> {
    // check if input is available
    if let Some(input) = image.0.take() {
        println!("Starting inference!");

        // not all ndarrays are contiguous in memory, but
        //  we know this one is thus we can unwrap safely
        let slice = input.as_slice_memory_order().unwrap();

        // run model inference
        match model.try_start_infer(slice) {
            Ok(()) => {}
            Err(_) => {
                // Whenever `try_infer` fails, it means
                //  the model inference has not finished yet
                //  since the previous call.
            }
        }
    }

    // check if output is available
    if let Some(output) = model.poll::<Vec<f32>>() {
        // note that inference might have failed
        let res = output?;

        println!("ResNet18 results: {res:?}");
    }

    Ok(())
}

fn main() -> Result<()> {
    let task_config = TaskConfig {
        async_threads: 1,
        compute_threads: 1,
    };

    App::new()
        .add_resource(Resource::new(task_config))?
        .add_module(TaskModule)?
        .add_module(MlModule)?
        // add the ML model
        .add_ml_task::<ResNet18>()?
        // use the ML model
        .add_system(process_image)
        .run()?;

    Ok(())
}
