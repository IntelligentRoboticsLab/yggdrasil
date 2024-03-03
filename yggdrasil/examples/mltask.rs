//! This is an example of how to implement a ML model.

use miette::Result;
use tyr::{system, App};
use yggdrasil::mltask::{MLModel, MLTask, MLTaskResource};

/// Imagine this implements the chatGPT 4.0 model.
struct ChatGPT4;

// implement MLModel to make it compatible with the rest of the system
impl MLModel for ChatGPT4 {
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
    model: &mut MLTask<ChatGPT4>,
    // model input
    prompt: &mut Prompt
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
                // We can only start inference if the previous
                //  task has been completed. Whenever `try_infer` fails, it means
                //  the model inference has not finished yet.
            }
        }
    }

    // check if output is available
    if let Some(output) = model.poll() {
        // note that inference might have failed
        let bytes = match output {
            Ok(bytes) => bytes,
            Err(e) => return Err(e.wrap_err("chatGPT failed to run!"))
        };

        // convert output to desired type
        let text = std::str::from_utf8(&bytes).unwrap();

        println!("chatGPT4: {text}");
    }

    Ok(())
}

fn main() -> Result<()> {
    App::new()
        // add the ML model
        .add_ml_task::<ChatGPT4>()?
        // use the ML model
        .add_system(process_chat)
        .run()?;

    Ok(())
}