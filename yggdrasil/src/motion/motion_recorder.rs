use crate::filter::button::HeadButtons;
use miette::Result;
use nidhogg::{
    types::{FillExt, JointArray},
    NaoControlMessage, NaoState,
};
// use tokio::io::AsyncBufReadExt;
use tyr::prelude::*;

use crate::motion::motion_manager::MotionManager;
use crate::motion::motion_types::MotionType;

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
    naostate: &mut NaoState,
) -> Result<()> {
    if head_button.middle.is_tapped() {
        println!("MOTION ACTIVATED");
        // println!("NaoState:\n {:?}", naostate.position);
        mmng.start_new_motion(MotionType::StandupStomach)
    } else if head_button.rear.is_tapped() {
        println!("MOTION SLOPPY");
        mmng.stop_motion();
        nao_control_message.stiffness = JointArray::<f32>::fill(-1.0);
    }
    Ok(())
}
