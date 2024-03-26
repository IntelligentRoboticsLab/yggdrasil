use std::time::Duration;

use crate::filter::button::HeadButtons;
use miette::Result;
use nidhogg::{
    types::{FillExt, JointArray},
    NaoControlMessage, NaoState,
};
// use tokio::io::AsyncBufReadExt;
use tyr::prelude::*;

use crate::motion::motion_manager::MotionManager;
use crate::motion::motion_types::{MotionType, Movement};

pub struct MotionRecorder;

impl Module for MotionRecorder {
    fn initialize(self, app: App) -> Result<App> {
        Ok(app.add_system(register_button_press))
        // .add_system(joint_locking_recorder)
        // .add_resource(Resource::new(RecordingResources {
        //     locked: false,
        //     total_keyframes: 0,
        //     keyframes: Vec::new(),
        // }))
    }
}

#[system]
fn register_button_press(
    head_button: &mut HeadButtons,
    mmng: &mut MotionManager,
    nao_control_message: &mut NaoControlMessage,
) -> Result<()> {
    if head_button.middle.is_tapped() {
        println!("MOTION ACTIVATED");
        // println!("-----------------\n{:?}\n\n", naostate.position);
        mmng.start_new_motion(MotionType::Test)
    } else if head_button.rear.is_tapped() {
        println!("MOTION SLOPPY");
        mmng.stop_motion();
        nao_control_message.stiffness = JointArray::<f32>::fill(0.0);
        // println!("-----------------\n{:?}\n\n", naostate.position);
    }
    Ok(())
}

// pub struct RecordingResources {
//     pub keyframes: Vec<Movement>,
//     pub locked: bool,
//     pub total_keyframes: u32,
// }

// async fn read_command() -> String {
//     let mut input = String::new();
//     let stdin = tokio::io::stdin();
//     // Create a buffered wrapper, which implements BufRead
//     let mut reader = tokio::io::BufReader::new(stdin);
//     // Take a stream of lines from this
//     let _ = reader.read_line(&mut input);

//     print!("{:?}", input);

//     input
// }

// fn dispatch_command(task: &mut AsyncTask<String>) -> Result<()> {
//     match task.try_spawn(read_command()) {
//         // Dispatched
//         Ok(_) => Ok(()),

//         Err(tyr::tasks::Error::AlreadyActive) => Ok(()),
//     }
// }

// #[system]
// fn joint_locking_recorder(
//     nao_state: &NaoState,
//     nao_control_message: &mut NaoControlMessage,
//     recordingresources: &mut RecordingResources,
//     task: &mut AsyncTask<String>,
// ) -> Result<()> {
//     dispatch_command(task)?;
//     let Some(command) = task.poll() else {
//         return Ok(());
//     };

//     let args: Vec<&str> = command.split(' ').collect();

//     match args[0] {
//         "lock" => match args[1] {
//             "all" => {
//                 nao_control_message.stiffness = JointArray::<f32>::fill(0.3);
//                 print!("LOCKED!");
//             }
//             _ => {}
//         },
//         "unlock" => match args[1] {
//             "all" => {
//                 nao_control_message.stiffness = JointArray::<f32>::fill(0.0);
//                 print!("Free movement!");
//             }
//             _ => {}
//         },
//         "keyframe" => {
//             let new_movement: Movement = Movement {
//                 target_position: nao_state.position.clone(),
//                 duration: Duration::new(1, 0),
//             };
//             recordingresources.keyframes.push(new_movement);
//             recordingresources.total_keyframes += 1;
//             print!(
//                 "Frame added; Total: {:?}",
//                 recordingresources.total_keyframes
//             )
//         }
//         "new" => {
//             recordingresources.keyframes.clear();
//             recordingresources.total_keyframes = 0;
//             print!("Motion Initialised!");
//         }
//         "print" => {
//             let motion_json = serde_json::to_string(&recordingresources.keyframes);
//             println!("{:?}", motion_json);
//         }
//         _ => {}
//     }
//     Ok(())
// }
