use miette::Result;
use std::time::{Duration, Instant};

use nidhogg::NaoState;
use tyr::prelude::*;

/// The threshold for a button to be considered pressed.
const BUTTON_ACTIVATION_THRESHOLD: f32 = 0.5;

/// Describes the time a button needs to be held down, in order to move to the [`ButtonState::Held`].
const BUTTON_HELD_DURATION_THRESHOLD: Duration = Duration::from_millis(500);

/// A module offering structured wrappers for each Nao button, derived from the raw [`NaoState`].
///
/// By allowing systems to depend only on necessary buttons, this design enhances the dependency graph's efficiency.
///
/// This module provides the following resources to the application:
/// - [`HeadButtons`]
/// - [`ChestButton`]
/// - [`LeftHandButtons`]
/// - [`RightHandButtons`]
/// - [`LeftFootButtons`]
/// - [`RightFootButtons`]
///
/// These resources include a [`ButtonState`], representing the button's current status.
pub struct ButtonFilter;

impl Module for ButtonFilter {
    fn initialize(self, app: App) -> Result<App> {
        app.add_system(button_filter)
            .init_resource::<HeadButtons>()?
            .init_resource::<ChestButton>()?
            .init_resource::<LeftHandButtons>()?
            .init_resource::<RightHandButtons>()?
            .init_resource::<LeftFootButtons>()?
            .init_resource::<RightFootButtons>()
    }
}

#[derive(Default, Debug)]
pub enum ButtonState {
    /// The button is not being pressed.
    #[default]
    Neutral,
    /// The button has been tapped, meaning it was just released.
    Tapped,
    /// The button is being pressed. [`Instant`] records the timestamp when the button was pressed.
    Pressed(Instant),
    /// The button is held down. [`Instant`] records the timestamp since the button is held.
    Held(Instant),
}

impl ButtonState {
    /// Tell whether the button is currently pressed down.
    pub fn is_pressed(&self) -> bool {
        !matches!(self, Self::Neutral | Self::Tapped)
    }

    /// Tell whether the button has been tapped, meaning it was just released.
    pub fn is_tapped(&self) -> bool {
        matches!(self, Self::Tapped)
    }

    /// Tell whether the button is currently being held down.
    pub fn is_held(&self) -> bool {
        matches!(self, Self::Held(_))
    }

    /// Get the next state based on whether the button is currently pressed down.
    pub fn next(&self, is_pressed: bool) -> Self {
        match (self, is_pressed) {
            (ButtonState::Neutral | ButtonState::Tapped, true) => Self::Pressed(Instant::now()),
            (ButtonState::Neutral, false) => Self::Neutral,
            (ButtonState::Tapped, false) => Self::Neutral,
            (ButtonState::Pressed(start), true) => {
                if Instant::now()
                    .checked_duration_since(*start)
                    .is_some_and(|duration| duration >= BUTTON_HELD_DURATION_THRESHOLD)
                {
                    Self::Held(Instant::now())
                } else {
                    Self::Pressed(*start)
                }
            }
            (ButtonState::Held(start), true) => Self::Held(*start),
            (ButtonState::Held(_) | ButtonState::Pressed(_), false) => Self::Tapped,
        }
    }
}

/// Struct containing [`states`](`ButtonState`) of the buttons on the Nao's head.
#[derive(Default, Debug)]
pub struct HeadButtons {
    /// Front button on the head of the Nao.
    pub front: ButtonState,
    /// Middle button on the head of the Nao.
    pub middle: ButtonState,
    /// Rear button on the head of the Nao.
    pub rear: ButtonState,
}

/// Struct containing [`state`](`ButtonState`) of the buttons in the Nao's chest.
#[derive(Default, Debug)]
pub struct ChestButton {
    /// The button in the chest of the Nao.
    pub state: ButtonState,
}

/// Struct containing [`states`](`ButtonState`) of the buttons on the Nao's left hand.
#[derive(Default, Debug)]
pub struct LeftHandButtons {
    /// Left button on the left hand of the Nao.
    pub left: ButtonState,
    /// Right button on the left hand of the Nao.
    pub right: ButtonState,
    /// Back button on the back of the left hand of the Nao.
    pub back: ButtonState,
}

/// Struct containing [`states`](`ButtonState`) of the buttons on the Nao's right hand.
#[derive(Default, Debug)]
pub struct RightHandButtons {
    /// Left button on the right hand of the Nao.
    pub left: ButtonState,
    /// Right button on the right hand of the Nao.
    pub right: ButtonState,
    /// Back button on the back of the right hand of the Nao.
    pub back: ButtonState,
}

/// Struct containing [`states`](`ButtonState`) of the buttons on the Nao's left foot.
#[derive(Default, Debug)]
pub struct LeftFootButtons {
    /// Left button on the left foot of the Nao.
    pub left: ButtonState,
    /// Right button on the left foot of the Nao.
    pub right: ButtonState,
}

/// Struct containing [`states`](`ButtonState`) of the buttons on the Nao's right foot.
#[derive(Default, Debug)]
pub struct RightFootButtons {
    /// Left button on the right foot of the Nao.
    pub left: ButtonState,
    /// Right button on the right foot of the Nao.
    pub right: ButtonState,
}

#[system]
fn button_filter(
    nao_state: &NaoState,
    head_buttons: &mut HeadButtons,
    chest_button: &mut ChestButton,
    left_hand_buttons: &mut LeftHandButtons,
    right_hand_buttons: &mut RightHandButtons,
    left_foot_buttons: &mut LeftFootButtons,
    right_foot_buttons: &mut RightFootButtons,
) -> Result<()> {
    head_buttons.front = head_buttons
        .front
        .next(nao_state.touch.head_front >= BUTTON_ACTIVATION_THRESHOLD);
    head_buttons.middle = head_buttons
        .middle
        .next(nao_state.touch.head_middle >= BUTTON_ACTIVATION_THRESHOLD);
    head_buttons.rear = head_buttons
        .rear
        .next(nao_state.touch.head_rear >= BUTTON_ACTIVATION_THRESHOLD);
    chest_button.state = chest_button
        .state
        .next(nao_state.touch.chest_board >= BUTTON_ACTIVATION_THRESHOLD);
    left_hand_buttons.left = left_hand_buttons
        .left
        .next(nao_state.touch.left_hand_left >= BUTTON_ACTIVATION_THRESHOLD);
    left_hand_buttons.right = left_hand_buttons
        .right
        .next(nao_state.touch.left_hand_right >= BUTTON_ACTIVATION_THRESHOLD);
    left_hand_buttons.back = left_hand_buttons
        .back
        .next(nao_state.touch.left_hand_back >= BUTTON_ACTIVATION_THRESHOLD);
    right_hand_buttons.left = right_hand_buttons
        .left
        .next(nao_state.touch.right_hand_left >= BUTTON_ACTIVATION_THRESHOLD);
    right_hand_buttons.right = right_hand_buttons
        .right
        .next(nao_state.touch.right_hand_right >= BUTTON_ACTIVATION_THRESHOLD);
    right_hand_buttons.back = right_hand_buttons
        .back
        .next(nao_state.touch.right_hand_back >= BUTTON_ACTIVATION_THRESHOLD);
    left_foot_buttons.left = left_foot_buttons
        .left
        .next(nao_state.touch.left_foot_left >= BUTTON_ACTIVATION_THRESHOLD);
    left_foot_buttons.right = left_foot_buttons
        .right
        .next(nao_state.touch.left_foot_right >= BUTTON_ACTIVATION_THRESHOLD);
    right_foot_buttons.left = right_foot_buttons
        .left
        .next(nao_state.touch.right_foot_left >= BUTTON_ACTIVATION_THRESHOLD);
    right_foot_buttons.right = right_foot_buttons
        .right
        .next(nao_state.touch.right_foot_right >= BUTTON_ACTIVATION_THRESHOLD);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn button_update() {
        let mut button = ButtonState::default();

        assert!(!button.is_tapped());
        assert!(!button.is_pressed());
        assert!(!button.is_held());

        button = button.next(true);

        assert!(!button.is_tapped());
        assert!(button.is_pressed());
        assert!(!button.is_held());

        std::thread::sleep(BUTTON_HELD_DURATION_THRESHOLD);
        button = button.next(true);

        assert!(!button.is_tapped());
        assert!(button.is_pressed());
        assert!(button.is_held());

        button = button.next(false);

        assert!(button.is_tapped());
        assert!(!button.is_pressed(),);
        assert!(!button.is_held(),);

        button = button.next(true);
        std::thread::sleep(BUTTON_HELD_DURATION_THRESHOLD / 2);
        button = button.next(true);

        assert!(!button.is_tapped());
        assert!(button.is_pressed());
        assert!(!button.is_held());

        button = button.next(false);

        assert!(button.is_tapped());
        assert!(!button.is_pressed());
        assert!(!button.is_held());
    }
}
