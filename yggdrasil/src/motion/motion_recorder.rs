use crate::filter::button::{HeadButtons, LeftFootButtons, RightFootButtons};
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
    head_button: &mut HeadButtons,
    left_button: &mut LeftFootButtons,
    right_button: &mut RightFootButtons,
) -> Result<()> {
    if left_button.left.is_tapped() {
        println!("Left Pressed!");
    }
    if right_button.left.is_tapped() {
        println!("Right Pressed!");
    }
    if head_button.front.is_tapped() {
        println!("Head Front Pressed!");
    }
    if head_button.middle.is_tapped() {
        println!("Head Middle Pressed!");
    }
    if head_button.rear.is_tapped() {
        println!("Head Rear Pressed!");
    }
    Ok(())
}
