use crate::filter::button::HeadButtons;
use miette::Result;
use nidhogg::NaoState;
use tyr::prelude::*;

pub struct MotionRecorder;

impl Module for MotionRecorder {
    fn initialize(self, app: App) -> Result<App> {
        Ok(app.add_system(register_button_press))
    }
}

#[system]
fn register_button_press(head_button: &mut HeadButtons, naostate: &NaoState) -> Result<()> {
    if head_button.middle.is_tapped() {
        println!("{:?}", naostate.position);
    }
    Ok(())
}
