use std::time::{Duration, Instant};

use color_eyre::Result;
use nidhogg::NaoState;
use tyr::prelude::*;

/// Describes the time a button needs to be held down, in order to move to the [`ButtonState::Held`].
const BUTTON_HELD_THRESHOLD: Duration = Duration::from_millis(500);

pub struct ButtonFilter;

impl Module for ButtonFilter {
    fn initialize(self, app: tyr::App) -> color_eyre::Result<tyr::App> {
        app.add_system(button_filter)
            .add_resource(Resource::new(HeadButtons::default()))?
            .add_resource(Resource::new(ChestButton::default()))
    }
}

#[derive(Default, Debug)]
pub enum ButtonState {
    /// State for when the button is not being pressed down.
    #[default]
    Neutral,
    /// State for when the button is being pressed down.
    Pressed(Instant),
    /// State for when the button is being held down, contains an [`Instant`] which is the timestamp since the button is held.
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

#[derive(Default)]
pub struct HeadButtons {
    pub front: ButtonState,
    pub middle: ButtonState,
    pub rear: ButtonState,
}

#[derive(Default)]
pub struct ChestButton(ButtonState);

#[system]
fn button_filter(
    nao_state: &NaoState,
    head_buttons: &mut HeadButtons,
    chest_button: &mut ChestButton,
) -> Result<()> {
    head_buttons.front = head_buttons.front.next(nao_state.touch.head_front > 0.0);
    head_buttons.middle = head_buttons.middle.next(nao_state.touch.head_middle > 0.0);
    head_buttons.rear = head_buttons.rear.next(nao_state.touch.head_rear > 0.0);
    chest_button.0 = chest_button.0.next(nao_state.touch.chest_board > 0.0);

    // TODO: rest of the touch sensors

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
