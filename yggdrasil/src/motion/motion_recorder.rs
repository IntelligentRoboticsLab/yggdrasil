use crate::filter::button::{LeftFootButtons, RightFootButtons};
use miette::Result;
use tyr::prelude::*;

pub struct Test;

impl Module for Test {
    fn initialize(self, app: App) -> Result<App> {
        Ok(app.add_system(register_button_press))
    }
}

#[system]
fn register_button_press(
    left_button: &mut LeftFootButtons,
    right_button: &mut RightFootButtons,
) -> Result<()> {
    if left_button.left.is_held() {
        println!("Left Pressed!");
    }
    if right_button.left.is_held() {
        println!("Right Pressed!");
    }
    Ok(())
}
