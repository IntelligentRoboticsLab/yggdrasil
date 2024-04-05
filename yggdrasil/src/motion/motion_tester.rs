use crate::filter::button::HeadButtons;
use miette::Result;
use nidhogg::{
    types::{FillExt, JointArray},
    NaoControlMessage,
};
// use tokio::io::AsyncBufReadExt;
use tyr::prelude::*;

use crate::motion::motion_manager::MotionManager;
use crate::motion::motion_types::MotionType;

pub struct MotionTester;

impl Module for MotionTester {
    fn initialize(self, app: App) -> Result<App> {
        Ok(app.add_system(debug_testmotion))
    }
}

#[system]
fn debug_testmotion(
    head_button: &mut HeadButtons,
    mmng: &mut MotionManager,
    nao_control_message: &mut NaoControlMessage,
) -> Result<()> {
    if head_button.middle.is_tapped() {
        println!("MOTION ACTIVATED");
        mmng.start_new_motion(MotionType::StandupStomach)
    } else if head_button.rear.is_tapped() {
        println!("MOTION SLOPPY");
        mmng.stop_motion();
        nao_control_message.stiffness = JointArray::<f32>::fill(-1.0);
    }
    Ok(())
}
