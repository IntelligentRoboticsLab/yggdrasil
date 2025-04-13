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

use std::fs;
use std::io::Write;

pub struct FootBumperPlugin;

impl Plugin for FootBumperPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Sensor, obstacle_detection)
            .init_resource::<ObstacleStateFromBumpers>()
            .init_resource::<FootBumperState>();
    }
}

/// Represents the current and previous state of obstacle detection, based on the foot bumpers.
#[derive(Resource, Debug)]
pub struct ObstacleStateFromBumpers {
    current_state: ObstacleStatus,
    prev_state: ObstacleStatus,
    pub enabled: bool,
    // obstacle_count: 0,
    // obstacle_start_time: None,
}

impl Default for ObstacleStateFromBumpers {
    fn default() -> Self {
        Self {
            current_state: ObstacleStatus::NotDetected,
            prev_state: ObstacleStatus::NotDetected,
            enabled: true,
        }
    }
}

impl ObstacleStateFromBumpers {
    /// Get the next obstacle state, based on the foot bumper data.
    pub fn update_state(&mut self, config: &FootBumperConfig, footbumper: &FootBumperState) {
        self.prev_state = self.current_state;

        let left_sum = footbumper.left_outer_count + footbumper.left_inner_count;
        let right_sum = footbumper.right_outer_count + footbumper.right_inner_count;
        let inner_sum = footbumper.left_inner_count + footbumper.right_inner_count;

        if inner_sum >= config.min_detection_count
            || left_sum >= config.min_detection_count && right_sum >= config.min_detection_count
        {
            self.current_state = ObstacleStatus::Middle;
        } else if left_sum >= config.min_detection_count {
            self.current_state = ObstacleStatus::Left;
        } else if right_sum >= config.min_detection_count {
            self.current_state = ObstacleStatus::Right;
        } else {
            self.current_state = ObstacleStatus::NotDetected;
        }

        // Interpret Left <-> Right transition as Middle.
        self.current_state = match (self.prev_state, self.current_state) {
            (ObstacleStatus::Left, ObstacleStatus::Right) |
            (ObstacleStatus::Right, ObstacleStatus::Left) => ObstacleStatus::Middle,
            _ => self.current_state,
        };
    }

    /// Whether an obstacle was just detected on the left.
    #[must_use]
    pub fn new_obstacle_left(&self) -> bool {
        !matches!(self.prev_state, ObstacleStatus::Left)
            && matches!(self.current_state, ObstacleStatus::Left)
    }

    /// Whether an obstacle was just detected on the right.
    #[must_use]
    pub fn new_obstacle_right(&self) -> bool {
        !matches!(self.prev_state, ObstacleStatus::Right)
            && matches!(self.current_state, ObstacleStatus::Right)
    }

    /// Whether an obstacle was just detected in the middle.
    #[must_use]
    pub fn new_obstacle_middle(&self) -> bool {
        !matches!(self.prev_state, ObstacleStatus::Middle)
            && matches!(self.current_state, ObstacleStatus::Middle)
    }
}

/// Represents the current status of obstacle detection, based on the foot bumpers.
#[derive(Default, Debug, Clone, Copy, PartialEq)]
pub enum ObstacleStatus {
    #[default] // Maybe bit double as default is manually set in foot bumpers (?)
    NotDetected,
    Left,
    Right,
    Middle,
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
    // Counts the consecutive bumps and resets after a certain time of no detections.
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
    debug_file: std::fs::File, // Remove later
}

impl Default for FootBumperState {
    fn default() -> Self {
        FootBumperState {
            left_outer_count: 0,
            left_inner_count: 0,
            right_outer_count: 0,
            right_inner_count: 0,
            // left_contact_buffer: VecDeque::from(vec![false; 25]),    // to do make config
            // right_contact_buffer: VecDeque::from(vec![false; 25]),   // to do make config
            left_prev_bump_time: None,
            right_prev_bump_time: None,
            debug_file: fs::File::options()
                .write(true)
                .create(true)
                .open("bumpers_1500_70.txt")
                .unwrap(), // remove later
        }
    }
}

impl FootBumperState {
    pub fn update_bumper_data(
        &mut self,
        config: &FootBumperConfig,
        left_foot: &LeftFootButtons,
        right_foot: &RightFootButtons,
    ) {
        // Change into bool
        let left_outer = left_foot.left.is_pressed();
        let left_inner = left_foot.right.is_pressed();
        let right_outer = right_foot.right.is_pressed();
        let right_inner = right_foot.left.is_pressed();

        // println!("left_outer = {}", self.left_outer_count);
        // println!("left_inner = {}", self.left_inner_count);
        // println!("right_outer = {}", self.right_outer_count);
        // println!("right_inner = {}", self.right_inner_count);
        // println!("----------------");

        if left_outer || left_inner {
            if left_outer {
                self.left_outer_count += 1;
            }
            if left_inner {
                self.left_inner_count += 1;
            }
            self.left_prev_bump_time = Some(Instant::now());
        }

        if right_outer || right_inner {
            if right_outer {
                self.right_outer_count += 1;
            }
            if right_inner {
                self.right_inner_count += 1;
            }
            self.right_prev_bump_time = Some(Instant::now());
        }

        // Resets left inner and left outer bumper data
        if let Some(left_prev_bump_time) = self.left_prev_bump_time {
            if left_prev_bump_time.elapsed() >= config.max_inactivity_time {
                self.left_outer_count = 0;
                self.left_inner_count = 0;
                self.left_prev_bump_time = None;
            }
        }

        // Resets right inner and right outer bumper data
        if let Some(right_prev_bump_time) = self.right_prev_bump_time {
            if right_prev_bump_time.elapsed() >= config.max_inactivity_time {
                self.right_outer_count = 0;
                self.right_inner_count = 0;
                self.right_prev_bump_time = None;
            }
        }

        // Will remove later
        writeln!(
            self.debug_file,
            "{:?}, {:?}, {:?}, {:?},",
            self.left_outer_count,
            self.left_inner_count,
            self.right_outer_count,
            self.right_inner_count
        )
        .unwrap();

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
    obstacle_state.update_state(config, &footbumpers);

    if obstacle_state.new_obstacle_left()
        || obstacle_state.new_obstacle_right()
        || obstacle_state.new_obstacle_middle()
    {
        println!("new_obstacle_left = {}", obstacle_state.new_obstacle_left());
        println!(
            "new_obstacle_right = {}",
            obstacle_state.new_obstacle_right()
        );
        println!(
            "new_obstacle_middle = {}",
            obstacle_state.new_obstacle_middle()
        );
        println!("----------------");
    }

    // Show current object detection state from the foot bumpers by showing lights.
    // Maybe should change when they light up.
    match obstacle_state.current_state {
        ObstacleStatus::Left => {
            manager.set_left_foot_led(color::f32::BLUE, Priority::Critical);
            manager.set_right_foot_led(color::f32::EMPTY, Priority::Critical);
        }
        ObstacleStatus::Right => {
            manager.set_right_foot_led(color::f32::BLUE, Priority::Critical);
            manager.set_left_foot_led(color::f32::EMPTY, Priority::Critical);
        }
        ObstacleStatus::Middle => {
            manager.set_left_foot_led(color::f32::BLUE, Priority::Critical);
            manager.set_right_foot_led(color::f32::BLUE, Priority::Critical);
        }
        ObstacleStatus::NotDetected => {
            manager.set_left_foot_led(color::f32::EMPTY, Priority::Critical);
            manager.set_right_foot_led(color::f32::EMPTY, Priority::Critical);
        }
    }
}

// Maybe nicer to have this as impl of footbumperstate(?)
// fn check_working_bumpers(
// Count based on buffer
// let left_buffer_count = self.left_contact_buffer.iter().filter(|&&x| x).count() as i32;
// let right_buffer_count = self.right_contact_buffer.iter().filter(|&&x| x).count() as i32;
// Push and pop the buffer
// self.left_contact_buffer.push_back(left_outer || left_inner);
// self.left_contact_buffer.pop_front();
// self.right_contact_buffer.push_back(right_outer || right_inner);
// self.right_contact_buffer.pop_front();
// )
