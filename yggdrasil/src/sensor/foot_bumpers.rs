use super::SensorConfig;
use super::button::{LeftFootButtons, RightFootButtons};
use crate::behavior::behaviors::Standup;
use crate::behavior::engine::in_behavior;
use crate::prelude::*;
use crate::{
    motion::path_finding::Obstacle,
    motion::step_planner::{DynamicObstacle, StepPlanner},
    nao::{NaoManager, Priority},
};
use bevy::prelude::*;
use nalgebra::Point2;
use nidhogg::types::color;
use serde::{Deserialize, Serialize};
use serde_with::{DurationMilliSeconds, serde_as};
use std::time::{Duration, Instant};

pub struct FootBumperPlugin;

impl Plugin for FootBumperPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Sensor,
            obstacle_detection.run_if(not(in_behavior::<Standup>)),
        )
        .init_resource::<ObstacleStateFromBumpers>()
        .init_resource::<FootBumperValues>();
    }
}

/// Represents the current and previous state of obstacle detection, based on the foot bumpers.
#[derive(Resource, Debug, Default)]
pub struct ObstacleStateFromBumpers {
    current_state: ObstacleStatus,
    prev_state: ObstacleStatus,
}

impl ObstacleStateFromBumpers {
    /// Get the next obstacle state, based on the foot bumper values.
    fn update_state(&mut self, config: &FootBumperConfig, foot_bumper: &FootBumperValues) {
        self.prev_state = self.current_state;

        let left_sum = foot_bumper.left_outer_count + foot_bumper.left_inner_count;
        let right_sum = foot_bumper.right_outer_count + foot_bumper.right_inner_count;
        let inner_sum = foot_bumper.left_inner_count + foot_bumper.right_inner_count;

        let left_detected = left_sum >= config.min_detection_count && !foot_bumper.left_inactive;
        let right_detected = right_sum >= config.min_detection_count && !foot_bumper.right_inactive;
        let inner_detected = inner_sum >= config.min_detection_count
            && !foot_bumper.left_inactive
            && !foot_bumper.right_inactive
            && left_sum > 0
            && right_sum > 0;

        self.current_state = if inner_detected || (left_detected && right_detected) {
            ObstacleStatus::Middle
        } else if left_detected {
            ObstacleStatus::Left
        } else if right_detected {
            ObstacleStatus::Right
        } else {
            ObstacleStatus::NotDetected
        };

        // Interpret L <-> R transition as an obstacle in the middle.
        self.current_state = match (self.prev_state, self.current_state) {
            (ObstacleStatus::Left, ObstacleStatus::Right)
            | (ObstacleStatus::Right, ObstacleStatus::Left) => ObstacleStatus::Middle,
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

/// The possible obstacle states.
#[derive(Default, Debug, Clone, Copy, PartialEq)]
pub enum ObstacleStatus {
    #[default]
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
    pub min_detection_count: u32,
    /// Time of no contact after which the foot bumper values will be reset.
    #[serde_as(as = "DurationMilliSeconds<u64>")]
    pub max_inactivity_time: Duration,
    /// Minimum number of pressure detections needed for one bumper to consider it malfunctioning.
    pub malfunction_count: u32,
    /// Angle that is used in spawning an obstacle on the left or right.
    pub obstacle_angle: f32,
    /// Distance from robot to obstacle that is used in spawning an obstacle.
    pub obstacle_distance: f32,
    /// Radius of an obstacle that will be spawned.
    pub obstacle_radius: f32,
    /// Merge distance used in spawning obstacles.
    pub merge_distance: f32,
    /// Time-to-live of an obstacle that will be spawned.
    #[serde_as(as = "DurationMilliSeconds<u64>")]
    pub ttl: Duration,
}

#[derive(Resource, Debug, Default)]
pub struct FootBumperValues {
    // Counts bumps. It resets to 0 after a certain time of no detections.
    left_outer_count: u32,
    left_inner_count: u32,
    right_outer_count: u32,
    right_inner_count: u32,
    // Whether the foot bumpers should be ignored.
    left_inactive: bool,
    right_inactive: bool,
    // Timestamp at the last detected contact.
    left_prev_bump_time: Option<Instant>,
    right_prev_bump_time: Option<Instant>,
}

impl FootBumperValues {
    pub fn update_bumper_values(
        &mut self,
        config: &FootBumperConfig,
        left_foot: &LeftFootButtons,
        right_foot: &RightFootButtons,
    ) {
        self.ignore_foot(config);

        let left_outer = left_foot.left.is_pressed();
        let left_inner = left_foot.right.is_pressed();
        let right_outer = right_foot.right.is_pressed();
        let right_inner = right_foot.left.is_pressed();

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

        // Reset bumper values after inactivity.
        if let Some(left_prev_bump_time) = self.left_prev_bump_time {
            if left_prev_bump_time.elapsed() >= config.max_inactivity_time {
                self.left_outer_count = 0;
                self.left_inner_count = 0;
                self.left_prev_bump_time = None;
            }
        }

        if let Some(right_prev_bump_time) = self.right_prev_bump_time {
            if right_prev_bump_time.elapsed() >= config.max_inactivity_time {
                self.right_outer_count = 0;
                self.right_inner_count = 0;
                self.right_prev_bump_time = None;
            }
        }
    }

    /// Sets foot bumpers inactive if they appear to be constantly in a pressed state,
    /// and back to active if they get out of the constantly pressed state.
    fn ignore_foot(&mut self, config: &FootBumperConfig) {
        self.left_inactive = self.left_inner_count >= config.malfunction_count
            || self.left_outer_count >= config.malfunction_count;
        self.right_inactive = self.right_inner_count >= config.malfunction_count
            || self.right_outer_count >= config.malfunction_count;
    }
}

fn set_foot_leds(manager: &mut NaoManager, current_state: ObstacleStatus) {
    match current_state {
        ObstacleStatus::Left => {
            manager.set_left_foot_led(color::f32::BLUE, Priority::Critical);
            manager.set_right_foot_led(color::f32::EMPTY, Priority::Critical);
        }
        ObstacleStatus::Right => {
            manager.set_left_foot_led(color::f32::EMPTY, Priority::Critical);
            manager.set_right_foot_led(color::f32::BLUE, Priority::Critical);
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

fn obstacle_detection(
    config: Res<SensorConfig>,
    mut foot_bumpers: ResMut<FootBumperValues>,
    mut obstacle_state: ResMut<ObstacleStateFromBumpers>,
    mut manager: ResMut<NaoManager>,
    mut step_planner: ResMut<StepPlanner>,
    left_foot: Res<LeftFootButtons>,
    right_foot: Res<RightFootButtons>,
) {
    let config = &config.foot_bumpers;
    foot_bumpers.update_bumper_values(config, &left_foot, &right_foot);
    obstacle_state.update_state(config, &foot_bumpers);
    set_foot_leds(&mut manager, obstacle_state.current_state);

    if obstacle_state.new_obstacle_left()
        || obstacle_state.new_obstacle_right()
        || obstacle_state.new_obstacle_middle()
    {
        let angle = if obstacle_state.new_obstacle_left() {
            config.obstacle_angle
        } else if obstacle_state.new_obstacle_right() {
            -config.obstacle_angle
        } else {
            0.0
        };

        let relative_pos = Point2::new(
            config.obstacle_distance * angle.cos(),
            config.obstacle_distance * angle.sin(),
        );

        let obstacle = DynamicObstacle {
            obs: Obstacle::new(relative_pos.x, relative_pos.y, config.obstacle_radius),
            ttl: Instant::now() + config.ttl,
        };

        step_planner.add_dynamic_obstacle(obstacle, config.merge_distance);
    }
}
