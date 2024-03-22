use crate::filter::button::HeadButtons;
use miette::Result;
use nidhogg::NaoState;
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
fn register_button_press(head_button: &mut HeadButtons, mmng: &mut MotionManager) -> Result<()> {
    if head_button.middle.is_tapped() {
        // println!("-----------------\n{:?}\n\n", naostate.position);
        mmng.start_new_motion(MotionType::Test)
    }
    Ok(())
}
