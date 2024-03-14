use yggdrasil::filter::button::{LeftFootButtons, RightFootButtons, HeadButtons};
use miette::Result;
use yggdrasil::prelude::*;

// I have setup a connection to Lola, however it it necessary to write a message to the backend very 12ms. 
// The write call to the backend end is blocking. So if you make a loop and write in it every iteration, that should work.
// The difficult thing wil be, reading user input, but not blocking on it and still making sure messages are sent. 

// Dario note: We will need to add threads, async, etc. USE TOKIO, WATCH TUTORIALS


pub struct Sk;

impl Module for Sk {
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
