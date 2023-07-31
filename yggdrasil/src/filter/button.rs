use std::time::{Duration, Instant};

use miette::Result;
use nidhogg::NaoState;
use tyr::prelude::*;

/// Describes the time a button needs to be held down, in order to move to the [`ButtonState::Held`].
const BUTTON_HELD_THRESHOLD: Duration = Duration::from_millis(500);

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
            .add_resource(Resource::new(HeadButtons::default()))?
            .add_resource(Resource::new(ChestButton::default()))?
            .add_resource(Resource::new(LeftHandButtons::default()))?
            .add_resource(Resource::new(RightHandButtons::default()))?
            .add_resource(Resource::new(LeftFootButtons::default()))?
            .add_resource(Resource::new(RightFootButtons::default()))
    }
}

#[derive(Default, Debug)]
pub enum ButtonState {
    /// The button is not being pressed.
    #[default]
    Neutral,
    /// The button is being pressed. [`Instant`] records the timestamp when the button was pressed.
    Pressed(Instant),
    /// The button is held down. [`Instant`] records the timestamp since the button is held.
    Held(Instant),
}

impl ButtonState {
    /// Tell whether the button is currently pressed down.
    pub fn is_pressed(&self) -> bool {
        !matches!(self, Self::Neutral)
    }

    /// Tell whether the button is currently being held down.
    pub fn is_held(&self) -> bool {
        matches!(self, Self::Held(_))
    }

    /// Get the next state based on whether the button is currently pressed down.
    pub fn next(&self, is_pressed: bool) -> Self {
        match (self, is_pressed) {
            (ButtonState::Neutral, true) => Self::Pressed(Instant::now()),
            (ButtonState::Neutral, false) => Self::Neutral,
            (ButtonState::Pressed(start), true) => {
                if Instant::now()
                    .checked_duration_since(*start)
                    .is_some_and(|duration| duration >= BUTTON_HELD_THRESHOLD)
                {
                    Self::Held(Instant::now())
                } else {
                    Self::Pressed(*start)
                }
            }
            (ButtonState::Pressed(_), false) => Self::Neutral,
            (ButtonState::Held(start), true) => Self::Held(*start),
            (ButtonState::Held(_), false) => Self::Neutral,
        }
    }
}

/// Struct containing [`states`](`ButtonState`) of the buttons on the Nao's head.
#[derive(Default)]
pub struct HeadButtons {
    /// Front button on the head of the Nao.
    pub front: ButtonState,
    /// Middle button on the head of the Nao.
    pub middle: ButtonState,
    /// Rear button on the head of the Nao.
    pub rear: ButtonState,
}

wrap!(ChestButton, ButtonState);

/// Struct containing [`states`](`ButtonState`) of the buttons on the Nao's left hand.
#[derive(Default)]
pub struct LeftHandButtons {
    /// Left button on the left hand of the Nao.
    pub left: ButtonState,
    /// Right button on the left hand of the Nao.
    pub right: ButtonState,
    /// Back button on the back of the left hand of the Nao.
    pub back: ButtonState,
}

/// Struct containing [`states`](`ButtonState`) of the buttons on the Nao's right hand.
#[derive(Default)]
pub struct RightHandButtons {
    /// Left button on the right hand of the Nao.
    pub left: ButtonState,
    /// Right button on the right hand of the Nao.
    pub right: ButtonState,
    /// Back button on the back of the right hand of the Nao.
    pub back: ButtonState,
}

/// Struct containing [`states`](`ButtonState`) of the buttons on the Nao's left foot.
#[derive(Default)]
pub struct LeftFootButtons {
    /// Left button on the left foot of the Nao.
    pub left: ButtonState,
    /// Right button on the left foot of the Nao.
    pub right: ButtonState,
}

/// Struct containing [`states`](`ButtonState`) of the buttons on the Nao's right foot.
#[derive(Default)]
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
    head_buttons.front = head_buttons.front.next(nao_state.touch.head_front > 0.0);
    head_buttons.middle = head_buttons.middle.next(nao_state.touch.head_middle > 0.0);
    head_buttons.rear = head_buttons.rear.next(nao_state.touch.head_rear > 0.0);
    chest_button.0 = chest_button.0.next(nao_state.touch.chest_board > 0.0);
    left_hand_buttons.left = left_hand_buttons
        .left
        .next(nao_state.touch.left_hand_left > 0.0);
    left_hand_buttons.right = left_hand_buttons
        .right
        .next(nao_state.touch.left_hand_right > 0.0);
    left_hand_buttons.back = left_hand_buttons
        .back
        .next(nao_state.touch.left_hand_back > 0.0);
    right_hand_buttons.left = right_hand_buttons
        .left
        .next(nao_state.touch.right_hand_left > 0.0);
    right_hand_buttons.right = right_hand_buttons
        .right
        .next(nao_state.touch.right_hand_right > 0.0);
    right_hand_buttons.back = right_hand_buttons
        .back
        .next(nao_state.touch.right_hand_back > 0.0);
    left_foot_buttons.left = left_foot_buttons
        .left
        .next(nao_state.touch.left_foot_left > 0.0);
    left_foot_buttons.right = left_foot_buttons
        .right
        .next(nao_state.touch.left_foot_right > 0.0);
    right_foot_buttons.left = right_foot_buttons
        .left
        .next(nao_state.touch.right_foot_left > 0.0);
    right_foot_buttons.right = right_foot_buttons
        .right
        .next(nao_state.touch.right_foot_right > 0.0);

    Ok(())
}

mod tests {

    #[test]
    fn button_update() {
        let mut button = crate::filter::button::ButtonState::default();

        assert!(
            !button.is_pressed(),
            "Button should initialize with `is_pressed == false`"
        );
        assert!(
            !button.is_held(),
            "Button should initialize with `is_held == false`"
        );

        button = button.next(true);

        assert!(
            button.is_pressed(),
            "Button should have `is_pressed == true` after update!"
        );
        assert!(
            !button.is_held(),
            "Button should have `is_held == false` after single update!"
        );

        std::thread::sleep(super::BUTTON_HELD_THRESHOLD);
        button = button.next(true);

        assert!(
            button.is_pressed(),
            "Button should have `is_pressed == true` after update!"
        );
        assert!(
            button.is_held(),
            "Button should have `is_held == true` after `BUTTON_HELD_THRESHHOLD` has passed!"
        );

        button = button.next(false);
        assert!(
            !button.is_pressed(),
            "Button should have `is_pressed == false` after no longer pressed!"
        );
        assert!(
            !button.is_held(),
            "Button should have `is_held == false` after no longer pressed!"
        );

        button = button.next(true);
        std::thread::sleep(super::BUTTON_HELD_THRESHOLD / 2);
        button = button.next(true);

        assert!(
            button.is_pressed(),
            "Button should have `is_pressed == true` after update!"
        );
        assert!(
            !button.is_held(),
            "Button should have `is_held == false` after `BUTTON_HELD_THRESHHOLD / 2` has passed!"
        );

        button = button.next(false);
        assert!(
            !button.is_pressed(),
            "Button should have `is_pressed == false` after no longer pressed!"
        );
        assert!(
            !button.is_held(),
            "Button should have `is_held == false` after no longer pressed!"
        );
    }
}
