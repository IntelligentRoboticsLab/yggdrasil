use crate::prelude::*;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DurationMilliSeconds};

use super::SensorConfig;
use nidhogg::NaoState;
use std::time::{Duration, Instant};

/// Plugin that adds resources for structured wrappers for each button on the nao,
/// derived from the raw [`NaoState`].
///
/// These resources include a [`ButtonState`], representing the button's current status.
pub struct ButtonPlugin;

impl Plugin for ButtonPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Sensor, button_filter)
            .init_resource::<HeadButtons>()
            .init_resource::<ChestButton>()
            .init_resource::<LeftHandButtons>()
            .init_resource::<RightHandButtons>()
            .init_resource::<LeftFootButtons>()
            .init_resource::<RightFootButtons>();
    }
}

/// Configuration for the button sensitivity.
#[serde_as]
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct ButtonConfig {
    /// The threshold for a button to be considered pressed.
    pub activation_threshold: f32,
    /// The time (in ms) a button needs to be held down, in order to be considered held down.
    #[serde_as(as = "DurationMilliSeconds<u64>")]
    pub held_duration_threshold: Duration,
}

/// The state of a button.
#[derive(Default, Debug, Clone, Copy, PartialEq)]
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
    #[must_use]
    pub fn is_pressed(&self) -> bool {
        !matches!(self, Self::Neutral | Self::Tapped)
    }

    /// Tell whether the button has been tapped, meaning it was just released.
    #[must_use]
    pub fn is_tapped(&self) -> bool {
        matches!(self, Self::Tapped)
    }

    /// Tell whether the button is currently being held down.
    #[must_use]
    pub fn is_held(&self) -> bool {
        matches!(self, Self::Held(_))
    }

    /// Get the next state based on whether the button is currently pressed down.
    #[must_use]
    pub fn next(&self, config: &ButtonConfig, is_pressed: bool) -> Self {
        match (self, is_pressed) {
            (ButtonState::Pressed(start), true) => {
                if Instant::now()
                    .checked_duration_since(*start)
                    .is_some_and(|duration| duration >= config.held_duration_threshold)
                {
                    Self::Held(Instant::now())
                } else {
                    Self::Pressed(*start)
                }
            }
            (ButtonState::Neutral | ButtonState::Tapped, true) => Self::Pressed(Instant::now()),
            (ButtonState::Neutral | ButtonState::Tapped, false) => Self::Neutral,
            (ButtonState::Held(start), true) => Self::Held(*start),
            (ButtonState::Held(_) | ButtonState::Pressed(_), false) => Self::Tapped,
        }
    }
}

/// Struct containing [`states`](`ButtonState`) of the buttons on the Nao's head.
#[derive(Resource, Default, Debug)]
pub struct HeadButtons {
    /// Front button on the head of the Nao.
    pub front: ButtonState,
    /// Middle button on the head of the Nao.
    pub middle: ButtonState,
    /// Rear button on the head of the Nao.
    pub rear: ButtonState,
}

impl HeadButtons {
    /// Tell whether all buttons are tapped, meaning they were just released.
    #[must_use]
    pub fn all_tapped(&self) -> bool {
        self.front.is_tapped() && self.middle.is_tapped() && self.rear.is_tapped()
    }

    /// Tell whether all buttons are pressed.
    #[must_use]
    pub fn all_pressed(&self) -> bool {
        self.front.is_pressed() && self.middle.is_pressed() && self.rear.is_pressed()
    }

    /// Tell whether all buttons are held down.
    #[must_use]
    pub fn all_held(&self) -> bool {
        self.front.is_held() && self.middle.is_held() && self.rear.is_held()
    }
}

/// Struct containing [`state`](`ButtonState`) of the buttons in the Nao's chest.
#[derive(Resource, Default, Debug)]
pub struct ChestButton {
    /// The button in the chest of the Nao.
    pub state: ButtonState,
}

/// Struct containing [`states`](`ButtonState`) of the buttons on the Nao's left hand.
#[derive(Resource, Default, Debug)]
pub struct LeftHandButtons {
    /// Left button on the left hand of the Nao.
    pub left: ButtonState,
    /// Right button on the left hand of the Nao.
    pub right: ButtonState,
    /// Back button on the back of the left hand of the Nao.
    pub back: ButtonState,
}

/// Struct containing [`states`](`ButtonState`) of the buttons on the Nao's right hand.
#[derive(Resource, Default, Debug)]
pub struct RightHandButtons {
    /// Left button on the right hand of the Nao.
    pub left: ButtonState,
    /// Right button on the right hand of the Nao.
    pub right: ButtonState,
    /// Back button on the back of the right hand of the Nao.
    pub back: ButtonState,
}

/// Struct containing [`states`](`ButtonState`) of the buttons on the Nao's left foot.
#[derive(Resource, Default, Debug)]
pub struct LeftFootButtons {
    /// Left button on the left foot of the Nao.
    pub left: ButtonState,
    /// Right button on the left foot of the Nao.
    pub right: ButtonState,
}

/// Struct containing [`states`](`ButtonState`) of the buttons on the Nao's right foot.
#[derive(Resource, Default, Debug)]
pub struct RightFootButtons {
    /// Left button on the right foot of the Nao.
    pub left: ButtonState,
    /// Right button on the right foot of the Nao.
    pub right: ButtonState,
}

fn button_filter(
    nao_state: Res<NaoState>,
    config: Res<SensorConfig>,
    (
        mut head_buttons,
        mut chest_button,
        mut left_hand_buttons,
        mut right_hand_buttons,
        mut left_foot_buttons,
        mut right_foot_buttons,
    ): (
        ResMut<HeadButtons>,
        ResMut<ChestButton>,
        ResMut<LeftHandButtons>,
        ResMut<RightHandButtons>,
        ResMut<LeftFootButtons>,
        ResMut<RightFootButtons>,
    ),
) {
    let touch = nao_state.touch.clone();
    let config = &config.button;
    let threshold = config.activation_threshold;

    // Hand buttons
    head_buttons.front = head_buttons
        .front
        .next(config, touch.head_front >= threshold);
    head_buttons.middle = head_buttons
        .middle
        .next(config, touch.head_middle >= threshold);
    head_buttons.rear = head_buttons.rear.next(config, touch.head_rear >= threshold);

    // Chest buttons
    chest_button.state = chest_button
        .state
        .next(config, touch.chest_board >= threshold);

    // Left hand buttons
    left_hand_buttons.left = left_hand_buttons
        .left
        .next(config, touch.left_hand_left >= threshold);
    left_hand_buttons.right = left_hand_buttons
        .right
        .next(config, touch.left_hand_right >= threshold);
    left_hand_buttons.back = left_hand_buttons
        .back
        .next(config, touch.left_hand_back >= threshold);

    // Right hand buttons
    right_hand_buttons.left = right_hand_buttons
        .left
        .next(config, touch.right_hand_left >= threshold);
    right_hand_buttons.right = right_hand_buttons
        .right
        .next(config, touch.right_hand_right >= threshold);
    right_hand_buttons.back = right_hand_buttons
        .back
        .next(config, touch.right_hand_back >= threshold);

    // Left foot buttons
    left_foot_buttons.left = left_foot_buttons
        .left
        .next(config, touch.left_foot_left >= threshold);
    left_foot_buttons.right = left_foot_buttons
        .right
        .next(config, touch.left_foot_right >= threshold);

    // Right foot buttons
    right_foot_buttons.left = right_foot_buttons
        .left
        .next(config, touch.right_foot_left >= threshold);
    right_foot_buttons.right = right_foot_buttons
        .right
        .next(config, touch.right_foot_right >= threshold);
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::time::Duration;

    // Note that this is not an odal config, it's just here to make the tests work.
    const CONFIG: ButtonConfig = ButtonConfig {
        activation_threshold: 0.5,
        held_duration_threshold: Duration::from_millis(500),
    };

    #[test]
    fn button_update() {
        let mut button = ButtonState::default();

        assert!(!button.is_tapped());
        assert!(!button.is_pressed());
        assert!(!button.is_held());

        button = button.next(&CONFIG, true);

        assert!(!button.is_tapped());
        assert!(button.is_pressed());
        assert!(!button.is_held());

        std::thread::sleep(CONFIG.held_duration_threshold);
        button = button.next(&CONFIG, true);

        assert!(!button.is_tapped());
        assert!(button.is_pressed());
        assert!(button.is_held());

        button = button.next(&CONFIG, false);

        assert!(button.is_tapped());
        assert!(!button.is_pressed(),);
        assert!(!button.is_held(),);

        button = button.next(&CONFIG, true);
        std::thread::sleep(CONFIG.held_duration_threshold / 2);
        button = button.next(&CONFIG, true);

        assert!(!button.is_tapped());
        assert!(button.is_pressed());
        assert!(!button.is_held());

        button = button.next(&CONFIG, false);

        assert!(button.is_tapped());
        assert!(!button.is_pressed());
        assert!(!button.is_held());
    }
}
