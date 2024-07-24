//! This is a showcase of an example architecture
//! that a machine learning application within the framework could have.
//! The example runs if a valid image and model path are provided.

use std::time::Duration;

use miette::Result;
use tyr::{
    system,
    tasks::{TaskConfig, TaskModule},
    App, Resource,
};
use yggdrasil::core::ml::{self, MlModel, MlModule, MlTask, MlTaskResource};

/// Link here to your favorite 224x224 image.
const IMAGE_PATH: &str = "animalos-folder/cat.jpg";

struct ResNet18;

// implement MLModel to make it compatible with the rest of the system
impl MlModel for ResNet18 {
    type InputType = f32;
    type OutputType = f32;

    /// Link here to the .onnx model.
    const ONNX_PATH: &'static str = "ai-folder/resnet18.onnx";
}

struct Image(Option<Vec<f32>>);

#[system]
fn generate_input(input_img: &mut Image) -> Result<()> {
    if input_img.0.is_none() {
        println!("Generating input...");
        std::thread::sleep(Duration::from_secs(1));

        // load image
        let img = image::ImageReader::open(IMAGE_PATH)
            .unwrap()
            .decode()
            .unwrap();
        let vec = img.into_rgb32f().into_vec();

        let r = vec
            .iter()
            .enumerate()
            .filter_map(|(i, v)| (i % 3 == 0).then_some(v));
        let g = vec
            .iter()
            .enumerate()
            .filter_map(|(i, v)| (i % 3 == 1).then_some(v));
        let b = vec
            .iter()
            .enumerate()
            .filter_map(|(i, v)| (i % 3 == 2).then_some(v));

        // convert to correct format
        let nchw_vec = r.chain(g).chain(b).copied().collect::<Vec<_>>();

        input_img.0 = Some(nchw_vec);
    }

    Ok(())
}

#[system]
fn process_image(
    // ML model (note the MLTask wrapper)
    ml_task: &mut MlTask<ResNet18>,
    // model input
    input_img: &mut Image,
) -> Result<()> {
    // check if input is available
    if let Some(input) = input_img.0.take() {
        // run model inference
        match ml_task.try_start_infer(&input) {
            Ok(()) => println!("Starting inference!"),
            // ML inference has not finished yet
            //  since the previous call, this is no problemo
            Err(ml::Error::Tyr(tyr::tasks::Error::AlreadyActive)) => {}
            // any other errors we might want to deal with
            error => error?,
        }
    }

    // check if output is available
    if let Some(output) = ml_task.poll::<Vec<f32>>() {
        // note that inference might have failed
        let res = output?;

        // take class with highest probability
        let argmax = res
            .iter()
            .enumerate()
            .max_by(|(_, v0), (_, v1)| v0.total_cmp(v1))
            .unwrap()
            .0;

        // check here if the output is correct
        // https://deeplearning.cms.waikato.ac.nz/user-guide/class-maps/IMAGENET/
        println!("ResNet18 prediction: {argmax}");
    }

    Ok(())
}

fn main() -> Result<()> {
    let task_config = TaskConfig {
        async_threads: 1,
        compute_threads: 1,
    };

    App::new()
        // necessary to run compute tasks
        .add_resource(Resource::new(task_config))?
        .add_module(TaskModule)?
        .add_module(MlModule)?
        .add_resource(Resource::new(Image(None)))?
        // add the ML model
        .add_ml_task::<ResNet18>()?
        .add_system(generate_input)
        // use the ML model
        .add_system(process_image)
        .run()?;

    Ok(())
}
