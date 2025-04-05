use crate::prelude::*;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DurationMilliSeconds};
use std::time::{Duration, Instant};
// use std::collections::VecDeque;
use super::button::{LeftFootButtons, RightFootButtons};
use super::SensorConfig;
use crate::nao::{NaoManager, Priority};
use nidhogg::types::color;

pub struct FootBumperPlugin;

impl Plugin for FootBumperPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Sensor, obstacle_detection)
            .init_resource::<ObstacleStateFromBumpers>()
            .init_resource::<FootBumperState>();
    }
}

/// Represents the current state of obstacle detection, based on the foot bumpers.
#[derive(Resource, Default, Debug, Clone, Copy, PartialEq)]
pub enum ObstacleStateFromBumpers {
    #[default] // Maybe bit double as default is manually set in foot bumpers (?)
    NotDetected,
    Left,
    Right,
    Middle,
}

impl ObstacleStateFromBumpers {
    /// Whether an obstacle is detected.
    #[must_use]
    pub fn obstacle_detected(&self) -> bool {
        !matches!(self, Self::NotDetected)
    }

    #[must_use]
    /// Whether an obstacle on the left is detected.
    pub fn obstacle_detected_left(&self) -> bool {
        matches!(self, Self::Left)
    }

    #[must_use]
    /// Whether an obstacle on the right is detected.
    pub fn obstacle_detected_right(&self) -> bool {
        matches!(self, Self::Right)
    }

    #[must_use]
    /// Whether an obstacle in the middle is detected.
    pub fn obstacle_detected_middle(&self) -> bool {
        matches!(self, Self::Middle)
    }

    #[must_use]
    /// Get the next obstacle state, based on the foot bumper data.
    pub fn next(&self, config: &FootBumperConfig, footbumper: &FootBumperState) -> Self {
        let left_count = footbumper.left_outer_count + footbumper.left_inner_count;
        let right_count = footbumper.right_outer_count + footbumper.right_inner_count;

        if left_count >= config.min_detection_count && right_count >= config.min_detection_count {
            Self::Middle
        } else if left_count >= config.min_detection_count {
            Self::Left
        } else if right_count >= config.min_detection_count {
            Self::Right
        } else {
            Self::NotDetected
        }
    }
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct FootBumperConfig {
    /// Minimum number of pressure detections needed to consider it an obstacle.
    pub min_detection_count: i32,
    /// Time of no contact after which contact counts will be reset.
    #[serde_as(as = "DurationMilliSeconds<u64>")]
    pub max_inactivity_time: Duration,
}

#[derive(Resource, Debug)]
pub struct FootBumperState {
    // Current assignment of object detection
    // obstacle: ObstacleStateFromBumpers,
    // prev_obstacle: ObjectState,
    // obstacle_count: i32,
    // obstacle_start_time: Option<Instant>,
    // Contact counters, counting the consecutive bumps.
    // Resets after a certain time of no detections.
    left_outer_count: i32,
    left_inner_count: i32,
    right_outer_count: i32,
    right_inner_count: i32,
    // Contact buffers containing the history of bumps
    // left_contact_buffer: VecDeque<bool>,
    // right_contact_buffer: VecDeque<bool>,
    // Time since the last detected contact
    left_prev_bump_time: Option<Instant>,
    right_prev_bump_time: Option<Instant>,
    // Whether or not a bump was detected in the previous cycle
    left_prev_cycle_pressed: bool,
    right_prev_cycle_pressed: bool,
}

impl Default for FootBumperState {
    fn default() -> Self {
        FootBumperState {
            // obstacle: ObstacleStateFromBumpers::NotDetected,
            // prev_obstacle: ObjectState::NotDetected,
            // obstacle_count: 0,
            // obstacle_start_time: None,
            left_outer_count: 0, // Can maybe be simplified to only left & right
            left_inner_count: 0,
            right_outer_count: 0,
            right_inner_count: 0,
            // left_contact_buffer: VecDeque::from(vec![false; 25]),    // to do make config
            // right_contact_buffer: VecDeque::from(vec![false; 25]),   // to do make config
            left_prev_bump_time: None,
            right_prev_bump_time: None,
            left_prev_cycle_pressed: false, // Nothing is done with this yet
            right_prev_cycle_pressed: false, // Nothing is done with this yet
        }
    }
}

impl FootBumperState {
    pub fn update_bumper_data(
        &mut self,
        // _config: &FootBumperConfig,
        config: &FootBumperConfig,
        left_foot: &LeftFootButtons,
        right_foot: &RightFootButtons,
    ) {
        // Change 'pressed' or 'held' into bool
        let left_outer = left_foot.left.is_pressed();
        let left_inner = left_foot.right.is_pressed();
        let right_outer = right_foot.right.is_pressed();
        let right_inner = right_foot.left.is_pressed();

        // let left_outer = matches!(left_foot.left, ButtonState::Pressed(_) | ButtonState::Held(_));
        // let left_inner = matches!(left_foot.right, ButtonState::Pressed(_) | ButtonState::Held(_));
        // let right_outer = matches!(right_foot.right, ButtonState::Pressed(_) | ButtonState::Held(_));
        // let right_inner = matches!(right_foot.left, ButtonState::Pressed(_) | ButtonState::Held(_));

        if left_outer || left_inner {
            if left_outer {
                self.left_outer_count += 1;
            }
            if left_inner {
                self.left_inner_count += 1;
            }
            self.left_prev_bump_time = Some(Instant::now());
            self.left_prev_cycle_pressed = true;
        } else {
            self.left_prev_cycle_pressed = false;
        }

        if right_outer || right_inner {
            if right_outer {
                self.right_outer_count += 1;
            }
            if right_inner {
                self.right_inner_count += 1;
            }
            self.right_prev_cycle_pressed = true;
            self.right_prev_bump_time = Some(Instant::now());
        } else {
            self.right_prev_cycle_pressed = false;
        }

        // Resets left bumpers (inner and outer will be reset at the same time)
        if let Some(left_prev_bump_time) = self.left_prev_bump_time {
            if left_prev_bump_time.elapsed() >= config.max_inactivity_time {
                self.left_outer_count = 0;
                self.left_inner_count = 0;
                self.left_prev_bump_time = None;
                self.left_prev_cycle_pressed = false; // Is this one necessary (?)
            }
        }

        // Resets right bumpers (inner and outer will be reset at the same time)
        if let Some(right_prev_bump_time) = self.right_prev_bump_time {
            if right_prev_bump_time.elapsed() >= config.max_inactivity_time {
                self.right_outer_count = 0;
                self.right_inner_count = 0;
                self.right_prev_bump_time = None;
                self.right_prev_cycle_pressed = false; // Is this one necessary (?)
            }
        }

        // Count based on buffer
        // let left_buffer_count = self.left_contact_buffer.iter().filter(|&&x| x).count() as i32;
        // let right_buffer_count = self.right_contact_buffer.iter().filter(|&&x| x).count() as i32;
        // Push and pop the buffer
        // self.left_contact_buffer.push_back(left_outer || left_inner);
        // self.left_contact_buffer.pop_front();
        // self.right_contact_buffer.push_back(right_outer || right_inner);
        // self.right_contact_buffer.pop_front();
    }
}

fn obstacle_detection(
    config: Res<SensorConfig>,
    mut footbumpers: ResMut<FootBumperState>,
    mut obstacle_state: ResMut<ObstacleStateFromBumpers>,
    mut manager: ResMut<NaoManager>,
    left_foot: Res<LeftFootButtons>,
    right_foot: Res<RightFootButtons>,
) {
    let config = &config.footbumpers;
    footbumpers.update_bumper_data(config, &left_foot, &right_foot);
    let new_state = obstacle_state.next(config, &footbumpers); // Not sure about ref/deref

    // Set LEDs
    if *obstacle_state != new_state {
        match new_state {
            ObstacleStateFromBumpers::Left => {
                manager.set_left_foot_led(color::f32::BLUE, Priority::Critical);
            }
            ObstacleStateFromBumpers::Right => {
                manager.set_right_foot_led(color::f32::BLUE, Priority::Critical);
            }
            ObstacleStateFromBumpers::Middle => {
                manager.set_left_foot_led(color::f32::BLUE, Priority::Critical);
                manager.set_right_foot_led(color::f32::BLUE, Priority::Critical);
            }
            // To do: find out how to turn off the leds
            ObstacleStateFromBumpers::NotDetected => {
                manager.set_left_foot_led(color::f32::ORANGE, Priority::Critical);
                manager.set_right_foot_led(color::f32::ORANGE, Priority::Critical);
            }
        }
    }

    *obstacle_state = new_state;
}
