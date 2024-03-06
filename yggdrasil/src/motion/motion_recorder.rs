use crate::filter::button::{LeftFootButtons, RightFootButtons, HeadButtons};
use miette::Result;
use nidhogg::{NaoControlMessage, NaoState};
use tyr::prelude::*;

pub struct MotionRecorder;

impl Module for MotionRecorder {
    fn initialize(self, app: App) -> Result<App> {
        Ok(app.add_system(joint_locking_recorder))
    }
}

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
//             recordingresources.motion.movements.push(new_movement);
//             recordingresources.total_keyframes += 1;
//             print!(
//                 "Frame added; Total: {:?}",
//                 recordingresources.total_keyframes
//             )
//         }
//         "new" => {
//             recordingresources.motion.initial_position = nao_state.position.clone();
//             recordingresources.motion.movements.clear();
//             recordingresources.total_keyframes = 0;
//             print!("Motion Initialised!");
//         }
//         "print" => {
//             let motion_json = serde_json::to_string(&recordingresources.motion);
//             println!("{:?}", motion_json);
//         }
//         _ => {}
//     }
//     Ok(())
// }

#[system]
fn joint_locking_recorder(
    nao_state: &NaoState,
    nao_control_message: &mut NaoControlMessage,
    // recordingresources: &mut RecordingResources,
    headbutton: &HeadButtons
) -> Result<()> {Ok(())}