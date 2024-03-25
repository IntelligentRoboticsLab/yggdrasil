use crate::filter::button::HeadButtons;
use miette::Result;
use nidhogg::{
    types::{FillExt, JointArray},
    NaoControlMessage,
};
use tyr::prelude::*;

use crate::motion::motion_manager::MotionManager;
use crate::motion::motion_types::MotionType;

pub struct MotionRecorder;

impl Module for MotionRecorder {
    fn initialize(self, app: App) -> Result<App> {
        Ok(app.add_system(register_button_press))
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
