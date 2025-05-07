use super::SensorConfig;
use super::button::{LeftFootButtons, RightFootButtons};
use crate::prelude::*;
use crate::{
    localization::RobotPose,
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

use std::fs;
use std::io::Write;

pub struct FootBumperPlugin;

impl Plugin for FootBumperPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Sensor, obstacle_detection)
            .init_resource::<ObstacleStateFromBumpers>()
            .init_resource::<FootBumperValues>();
    }
}

/// Represents the current and previous state of obstacle detection, based on the foot bumpers.
#[derive(Resource, Debug)]
pub struct ObstacleStateFromBumpers {
    current_state: ObstacleStatus,
    prev_state: ObstacleStatus,
}

impl Default for ObstacleStateFromBumpers {
    fn default() -> Self {
        Self {
            current_state: ObstacleStatus::NotDetected,
            prev_state: ObstacleStatus::NotDetected,
        }
    }
}

impl ObstacleStateFromBumpers {
    /// Get the next obstacle state, based on the foot bumper values.
    pub fn update_state(&mut self, config: &FootBumperConfig, footbumper: &FootBumperValues) {
        self.prev_state = self.current_state;

        let left_sum = footbumper.left_outer_count + footbumper.left_inner_count;
        let right_sum = footbumper.right_outer_count + footbumper.right_inner_count;
        let inner_sum = footbumper.left_inner_count + footbumper.right_inner_count;

        let left_detected = left_sum >= config.min_detection_count && footbumper.left_active;
        let right_detected = right_sum >= config.min_detection_count && footbumper.right_active;
        let inner_detected = inner_sum >= config.min_detection_count
            && footbumper.left_active
            && footbumper.right_active
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

        // Interpret L <-> R transition as an object in the middle.
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

/// Represents the current status of obstacle detection, based on the foot bumpers.
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
    pub min_detection_count: i32,
    /// Time of no contact after which the foot bumper values will be reset.
    #[serde_as(as = "DurationMilliSeconds<u64>")]
    pub max_inactivity_time: Duration,
    /// Minimum number of pressure detections needed for one bumper to consider it malfunctioning.
    pub malfunction_count: i32,
    /// Angle that is used in spawning an obstacle on the left or right.
    pub object_angle: f32,
    /// Distance from robot to object that is used in spawning an obstacle.
    pub object_distance: f32,
    /// Radius of an object that will be spawned.
    pub object_radius: f32,
    /// Merge distance of an object that will be spawned.
    pub merge_distance: f32,
    /// Time-to-live of an object that will be spawned.
    #[serde_as(as = "DurationMilliSeconds<u64>")]
    pub ttl: Duration,
}

#[derive(Resource, Debug)]
pub struct FootBumperValues {
    // Counts bumps and resets to 0 after a certain time of no detections.
    left_outer_count: i32,
    left_inner_count: i32,
    right_outer_count: i32,
    right_inner_count: i32,
    // Whether the foot bumper will be in use.
    left_active: bool,
    right_active: bool,
    // Time since the last detected contact.
    left_prev_bump_time: Option<Instant>,
    right_prev_bump_time: Option<Instant>,
    debug_file: std::fs::File, // Remove later
}

impl Default for FootBumperValues {
    fn default() -> Self {
        FootBumperValues {
            left_outer_count: 0,
            left_inner_count: 0,
            right_outer_count: 0,
            right_inner_count: 0,
            left_active: true,
            right_active: true,
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

impl FootBumperValues {
    pub fn update_bumper_values(
        &mut self,
        config: &FootBumperConfig,
        left_foot: &LeftFootButtons,
        right_foot: &RightFootButtons,
    ) {
        self.ignore_foot(&config);

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

        // Reset bumper values after inactivity
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

        // Remove later
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

    /// Sets foot bumpers inactive if they appear to be constantly in a pressed state,
    /// and back to active if they get out of the constantly pressed state.
    pub fn ignore_foot(&mut self, config: &FootBumperConfig) {
        let left_malfunction = self.left_inner_count >= config.malfunction_count
            || self.left_outer_count >= config.malfunction_count;
        let right_malfunction = self.right_inner_count >= config.malfunction_count
            || self.right_outer_count >= config.malfunction_count;

        self.left_active = !left_malfunction;
        self.right_active = !right_malfunction;
    }
}

fn obstacle_detection(
    config: Res<SensorConfig>,
    mut footbumpers: ResMut<FootBumperValues>,
    mut obstacle_state: ResMut<ObstacleStateFromBumpers>,
    mut manager: ResMut<NaoManager>,
    mut step_planner: ResMut<StepPlanner>,
    robot_pose: Res<RobotPose>,
    left_foot: Res<LeftFootButtons>,
    right_foot: Res<RightFootButtons>,
) {
    let config = &config.footbumpers;
    footbumpers.update_bumper_values(config, &left_foot, &right_foot);
    obstacle_state.update_state(config, &footbumpers);

    // Set LEDs
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

    if obstacle_state.new_obstacle_left() {
        println!("new_obstacle_left = {}", obstacle_state.new_obstacle_left());
        let angle = config.object_angle;
        spawn_obstacle(
            &mut step_planner,
            &robot_pose,
            config.object_radius,
            angle,
            config.object_distance,
            config.merge_distance,
            config.ttl,
        );
    } else if obstacle_state.new_obstacle_right() {
        println!(
            "new_obstacle_right = {}",
            obstacle_state.new_obstacle_right()
        );
        let angle = -config.object_angle;
        spawn_obstacle(
            &mut step_planner,
            &robot_pose,
            config.object_radius,
            angle,
            config.object_distance,
            config.merge_distance,
            config.ttl,
        );
    } else if obstacle_state.new_obstacle_middle() {
        println!(
            "new_obstacle_middle = {}",
            obstacle_state.new_obstacle_middle()
        );
        spawn_obstacle(
            &mut step_planner,
            &robot_pose,
            config.object_radius,
            0.0,
            config.object_distance,
            config.merge_distance,
            config.ttl,
        );
    }
}

fn spawn_obstacle(
    step_planner: &mut StepPlanner,
    robot_pose: &RobotPose,
    radius: f32,
    angle: f32,
    distance: f32,
    merge_distance: f32,
    ttl: Duration,
) {
    // Split distance (from robot to the center of the object) into components.
    let delta_x = distance * angle.cos();
    let delta_y = distance * angle.sin();
    let point = Point2::new(delta_x, delta_y);
    // Translate and rotate so that it aligns with the robot's position.
    let world_pos = robot_pose.robot_to_world(&point);

    println!("robot x,y: {}", robot_pose.world_position());
    println!("dx, dy = {}, {}", delta_x, delta_y);
    println!("obstacle x,y {}, {}", world_pos.x, world_pos.y);
    println!("-----------------");

    let obstacle = DynamicObstacle {
        obs: Obstacle::new(world_pos.x, world_pos.y, radius),
        ttl: Instant::now() + ttl, // Is this necessary (???)
    };

    step_planner.add_dynamic_obstacle(obstacle, merge_distance);
}
