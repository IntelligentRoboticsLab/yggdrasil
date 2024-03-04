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
use yggdrasil::ml_task::{MlModel, MlModule, MlTask, MlTaskResource};

struct ChatGPT4;

// implement MLModel to make it compatible with the rest of the system
impl MlModel for ChatGPT4 {
    fn onnx_path() -> &'static str {
        "microsoft/gpt4.onnx"
    }

    fn input_shape() -> Vec<usize> {
        vec![256]
    }
}

struct Prompt(Option<String>);

#[system]
fn process_chat(
    // ML model (note the MLTask wrapper)
    model: &mut MlTask<ChatGPT4>,
    // model input
    prompt: &mut Prompt,
) -> Result<()> {
    // check if input is available
    if let Some(input) = prompt.0.take() {
        println!("user: {input}");

        // convert input to bytes
        let bytes = input.as_bytes();

        // run model inference
        match model.try_start_infer(bytes) {
            Ok(()) => {}
            Err(_) => {
                // Whenever `try_infer` fails, it means
                //  the model inference has not finished yet
                //  since the previous call.
            }
        }
    }

    // check if output is available
    if let Some(output) = model.poll() {
        // note that inference might have failed
        let bytes = match output {
            Ok(bytes) => bytes,
            Err(e) => return Err(e.wrap_err("chatGPT failed to run!")),
        };

        // convert output to desired type
        let text = std::str::from_utf8(&bytes).unwrap();

        println!("chatGPT4: {text}");
    }

    Ok(())
}

fn main() -> Result<()> {
    let task_config = TaskConfig {
        async_threads: 1,
        compute_threads: 1,
    };

    App::new()
        .add_resource(Resource::new(Prompt(Some("input".into()))))?
        .add_resource(Resource::new(task_config))?
        .add_module(TaskModule)?
        .add_module(MlModule)?
        // add the ML model
        .add_ml_task::<ChatGPT4>()?
        // use the ML model
        .add_system(process_chat)
        .run()?;

    Ok(())
}
