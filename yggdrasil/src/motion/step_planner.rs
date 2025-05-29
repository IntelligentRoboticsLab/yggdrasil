use super::{
    path_finding::{self, Obstacle},
    walking_engine::step::{self, Step},
};
use crate::{core::debug::DebugContext, localization::RobotPose, nao::Cycle};
use bevy::prelude::*;
use nalgebra::{Isometry, Point2, UnitComplex, Vector2};
use rerun::{FillMode, LineStrip3D};
use std::time::Instant;

const TURN_SPEED: f32 = 0.35;
const WALK_SPEED: f32 = 0.05;
const SIDE_SPEED: f32 = 0.035;

// Control parameters
const ATTRACTION_GAIN: f32 = 1.2;
const ROTATION_GAIN: f32 = 2.5;
const ANGLE_THRESHOLD_FOR_PURE_TURN: f32 = 0.8;

/// Plugin that adds systems and resources for planning robot steps.
pub(super) struct StepPlannerPlugin;

impl Plugin for StepPlannerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<StepPlanner>();
        app.add_systems(
            PostStartup,
            (setup_path_visualizer, setup_dynamic_obstacle_logging),
        );
        app.add_systems(PostUpdate, (log_planned_path, log_dynamic_obstacles));
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Target {
    pub position: Point2<f32>,
    pub rotation: Option<UnitComplex<f32>>,
}

impl From<Point2<f32>> for Target {
    fn from(position: Point2<f32>) -> Self {
        Target {
            position,
            rotation: None,
        }
    }
}

#[derive(Debug, Clone, Resource)]
pub struct StepPlanner {
    target: Option<Target>,
    reached_translation_target: bool,
    reached_rotation_target: bool,

    static_obstacles: Vec<Obstacle>,
    dynamic_obstacles: Vec<DynamicObstacle>,
}

impl Default for StepPlanner {
    fn default() -> Self {
        StepPlanner {
            target: None,
            reached_translation_target: false,
            reached_rotation_target: false,
            static_obstacles: vec![
                Obstacle::new(4.500, 1.1, 0.2),
                Obstacle::new(4.500, -1.1, 0.2),
                Obstacle::new(-4.500, 1.1, 0.2),
                Obstacle::new(-4.500, -1.1, 0.2),
            ],
            dynamic_obstacles: vec![],
        }
    }
}

impl StepPlanner {
    pub fn set_absolute_target(&mut self, target: Target) {
        self.target = Some(target);
        self.reached_translation_target = false;
        self.reached_rotation_target = false;
    }

    pub fn set_absolute_target_if_unset(&mut self, target: Target) {
        if self.target.is_none() {
            self.set_absolute_target(target);
        }
    }

    pub fn clear_target(&mut self) {
        self.target = None;
        self.reached_translation_target = false;
        self.reached_rotation_target = false;
    }

    #[must_use]
    pub fn current_absolute_target(&self) -> Option<&Target> {
        self.target.as_ref()
    }

    pub fn add_dynamic_obstacle(&mut self, obstacle: DynamicObstacle, merge_distance: f32) {
        match self
            .dynamic_obstacles
            .iter_mut()
            .find(|o| o.obs.distance(&obstacle.obs) <= merge_distance)
        {
            Some(o) => o.ttl = obstacle.ttl,
            None => self.dynamic_obstacles.push(obstacle),
        }
    }

    fn collect_and_gc_dynamic_obstacles(&mut self) -> Vec<Obstacle> {
        let now = Instant::now();

        self.dynamic_obstacles.retain(|obs| now < obs.ttl);
        self.dynamic_obstacles.iter().map(|obs| obs.obs).collect()
    }

    fn get_all_obstacles(&mut self) -> Vec<Obstacle> {
        let mut all_obstacles = self.static_obstacles.clone();
        all_obstacles.extend_from_slice(&self.collect_and_gc_dynamic_obstacles());

        all_obstacles
    }

    fn calc_path(&mut self, robot_pose: &RobotPose) -> Option<(Vec<Point2<f32>>, f32)> {
        let target_position = self.target?.position;
        let all_obstacles = self.get_all_obstacles();

        let abs_obstacles: Vec<_> = all_obstacles
            .iter()
            .map(|obs| {
                let abs_pos = robot_pose.robot_to_world(&Point2::new(obs.x.0, obs.y.0));

                Obstacle::new(abs_pos.x, abs_pos.y, obs.radius.0)
            })
            .collect();

        path_finding::find_path(robot_pose.world_position(), target_position, &abs_obstacles)
    }

    /// Compute desired velocity in robot's local frame based on waypoint
    fn compute_desired_velocity(
        robot_pose: &RobotPose,
        waypoint: Point2<f32>,
        target: &Target,
    ) -> Vector2<f32> {
        // Transform waypoint to robot's local frame
        let local_waypoint = robot_pose.inner.inverse_transform_point(&waypoint);
        let distance_to_waypoint = local_waypoint.coords.norm();

        // Always maintain some minimum velocity to prevent standing still
        if distance_to_waypoint < 0.001 {
            return Vector2::new(0.01, 0.0); // Small forward velocity
        }

        // Direction to waypoint in robot's local frame
        let direction = local_waypoint.coords / distance_to_waypoint;

        // Dynamic speed scaling based on distance
        let speed_scale = if distance_to_waypoint < 0.1 {
            // Very close - move slowly for precision
            0.3
        } else if distance_to_waypoint < 0.3 {
            // Close - moderate speed
            0.6
        } else if distance_to_waypoint < 0.8 {
            // Medium distance - ramp up speed
            0.8 + (distance_to_waypoint - 0.3) * 0.4
        } else {
            // Far - full speed
            1.0
        };

        // Desired velocity in robot's local frame
        direction * ATTRACTION_GAIN * speed_scale
    }

    /// Convert desired velocity to holonomic step commands
    fn velocity_to_step(
        desired_velocity: Vector2<f32>,
        robot_pose: &RobotPose,
        target: &Target,
        distance_to_target: f32,
    ) -> Step {
        let vel_x = desired_velocity.x;
        let vel_y = desired_velocity.y;

        // Angle to the velocity vector (in robot's frame)
        let vel_angle = vel_y.atan2(vel_x);
        let vel_magnitude = desired_velocity.norm();
        let angle_magnitude = vel_angle.abs();

        // PRIORITY: If angle is large, prioritize turning first
        if angle_magnitude > ANGLE_THRESHOLD_FOR_PURE_TURN && distance_to_target > 0.15 {
            // Pure rotation with slight side movement to maintain some progress
            return Step {
                forward: 0.0,
                left: SIDE_SPEED * vel_angle.signum() * 0.3,
                turn: TURN_SPEED * vel_angle.signum(), // Full speed turning
            };
        }

        // Special handling for very close distances
        if distance_to_target < 0.15 {
            // Ultra-precise omnidirectional movement
            let forward_component = (vel_x * 2.0).clamp(-WALK_SPEED * 0.6, WALK_SPEED * 0.6);
            let side_component = (vel_y * 2.5).clamp(-SIDE_SPEED, SIDE_SPEED);

            // Aggressive rotation alignment when close
            let turn_component = if let Some(target_rot) = target.rotation {
                let angle_diff = target_rot.angle() - robot_pose.world_rotation();
                // No scaling - use full rotation gain
                (angle_diff * ROTATION_GAIN).clamp(-TURN_SPEED, TURN_SPEED)
            } else {
                // Even without target rotation, align with movement direction
                (vel_angle * ROTATION_GAIN).clamp(-TURN_SPEED, TURN_SPEED)
            };

            return Step {
                forward: forward_component,
                left: side_component,
                turn: turn_component,
            };
        }

        // Normal movement strategies
        if angle_magnitude > 2.5 {
            // Target is behind us (> ~143 degrees)
            if distance_to_target < 0.5 {
                // Close enough to back up efficiently with aggressive turning
                Step {
                    forward: -WALK_SPEED * 0.7,
                    left: SIDE_SPEED * vel_angle.signum() * 0.5,
                    turn: TURN_SPEED * vel_angle.signum(), // Full speed turn
                }
            } else {
                // Too far to back up - aggressive turn with side movement
                Step {
                    forward: 0.0,
                    left: SIDE_SPEED * vel_angle.signum(),
                    turn: TURN_SPEED * vel_angle.signum(), // Full speed turn
                }
            }
        } else if angle_magnitude > 0.5 {
            // Target is to the side (> ~29 degrees)
            // Aggressive turning with movement
            let forward_component = WALK_SPEED * vel_angle.cos().max(0.0) * vel_magnitude * 0.6;
            let side_component = SIDE_SPEED * vel_angle.sin() * vel_magnitude;
            let turn_component =
                TURN_SPEED * vel_angle.signum() * (0.7 + 0.3 * (angle_magnitude / 1.2)); // Scale up with angle

            Step {
                forward: forward_component,
                left: side_component,
                turn: turn_component,
            }
        } else if angle_magnitude > 0.2 {
            // Target is slightly to the side (> ~11 degrees)
            // Blend movements with significant turning
            let cos_angle = vel_angle.cos();
            let sin_angle = vel_angle.sin();

            let forward_component = WALK_SPEED * cos_angle * vel_magnitude;
            let side_component = SIDE_SPEED * sin_angle * vel_magnitude;
            let turn_component = vel_angle * ROTATION_GAIN; // No reduction

            Step {
                forward: forward_component,
                left: side_component,
                turn: turn_component.clamp(-TURN_SPEED, TURN_SPEED),
            }
        } else {
            // Target is mostly ahead (< ~11 degrees)
            // Fast forward with quick corrections
            let forward_component = WALK_SPEED * vel_magnitude;
            let side_component = SIDE_SPEED * vel_angle * 3.0; // Amplify small corrections
            let turn_component = vel_angle * ROTATION_GAIN * 2.0; // Double gain for quick alignment

            Step {
                forward: forward_component,
                left: side_component.clamp(-SIDE_SPEED * 0.5, SIDE_SPEED * 0.5),
                turn: turn_component.clamp(-TURN_SPEED * 0.8, TURN_SPEED * 0.8),
            }
        }
    }

    fn plan_velocity_based(
        &self,
        robot_pose: &RobotPose,
        target: &Target,
        path: &[Point2<f32>],
    ) -> Option<Step> {
        // Check if we've reached the position target
        let robot_position = robot_pose.world_position();
        let distance_to_target = ((robot_position.x - target.position.x).powi(2)
            + (robot_position.y - target.position.y).powi(2))
        .sqrt();

        if distance_to_target < 0.05 && path.len() == 2 {
            // At target position - handle final rotation if needed
            if let Some(target_rot) = target.rotation {
                let angle_diff = target_rot.angle() - robot_pose.world_rotation();
                if angle_diff.abs() > 0.2 {
                    return Some(Step {
                        forward: 0.0,
                        left: 0.0,
                        turn: TURN_SPEED * angle_diff.signum(),
                    });
                }
            }
            return None; // Reached both position and rotation
        }

        // Select waypoint intelligently
        let waypoint = if path.len() > 2 {
            // For longer paths, look ahead based on current speed
            // This helps smooth out sharp corners
            let look_ahead_distance = 0.3; // 30cm lookahead
            let mut accumulated_distance = 0.0;
            let mut selected_waypoint = path[1];

            for i in 1..path.len() {
                if i > 1 {
                    let segment_distance = ((path[i].x - path[i - 1].x).powi(2)
                        + (path[i].y - path[i - 1].y).powi(2))
                    .sqrt();
                    accumulated_distance += segment_distance;
                }

                selected_waypoint = path[i];

                if accumulated_distance >= look_ahead_distance {
                    break;
                }
            }

            selected_waypoint
        } else if path.len() > 1 {
            path[1]
        } else {
            target.position
        };

        // Compute desired velocity in robot's local frame
        let desired_velocity = Self::compute_desired_velocity(robot_pose, waypoint, target);

        // Convert velocity to step commands
        let mut step =
            Self::velocity_to_step(desired_velocity, robot_pose, target, distance_to_target);

        // Intelligent safety check - ensure we're making meaningful progress
        let total_movement = step.forward.abs() + step.left.abs() + step.turn.abs() * 0.1;
        if total_movement < 0.005 {
            // We're not moving enough - determine why and fix it
            let local_target = robot_pose.inner.inverse_transform_point(&target.position);
            let angle_to_target = local_target.y.atan2(local_target.x);

            if angle_to_target.abs() > 0.5 {
                // Target is to the side - force a turn
                step = Step {
                    forward: 0.0,
                    left: 0.0,
                    turn: TURN_SPEED * angle_to_target.signum(),
                };
            } else {
                // Target is ahead - force forward movement
                step = Step {
                    forward: WALK_SPEED * 0.5,
                    left: 0.0,
                    turn: 0.0,
                };
            }
        }

        Some(step)
    }

    pub fn plan(&mut self, robot_pose: &RobotPose) -> Option<Step> {
        let target = self.target?;
        let (path, _total_walking_distance) = self.calc_path(robot_pose)?;

        // Use velocity-based planning
        if let Some(step) = self.plan_velocity_based(robot_pose, &target, &path) {
            // Update reached flags
            let robot_position = robot_pose.world_position();
            let distance = ((robot_position.x - target.position.x).powi(2)
                + (robot_position.y - target.position.y).powi(2))
            .sqrt();

            if distance < 0.05 && path.len() == 2 {
                self.reached_translation_target = true;

                if let Some(target_rot) = target.rotation {
                    let angle_diff = (target_rot.angle() - robot_pose.world_rotation()).abs();
                    if angle_diff < 0.2 {
                        self.reached_rotation_target = true;
                        return None;
                    }
                } else {
                    self.reached_rotation_target = true;
                    return None;
                }
            }

            return Some(step);
        }

        None
    }

    #[must_use]
    pub fn reached_target(&self) -> bool {
        self.reached_translation_target && self.reached_rotation_target
    }

    #[must_use]
    pub fn has_target(&self) -> bool {
        self.target.is_some()
    }
}

#[derive(Debug, Clone)]
pub struct DynamicObstacle {
    pub obs: Obstacle,
    pub ttl: Instant,
}

fn calc_turn(pose: &Isometry<f32, UnitComplex<f32>, 2>, target_point: Point2<f32>) -> f32 {
    let relative_transformed_target_point = pose.inverse_transform_point(&target_point);

    relative_transformed_target_point.y.signum() * TURN_SPEED
}

fn calc_angle_to_point(
    pose: &Isometry<f32, UnitComplex<f32>, 2>,
    target_point: Point2<f32>,
) -> f32 {
    let relative_transformed_target_point = pose.inverse_transform_point(&target_point);

    let relative_transformed_target_vector = Vector2::new(
        relative_transformed_target_point.x,
        relative_transformed_target_point.y,
    );

    relative_transformed_target_vector.angle(&Vector2::new(100., 0.))
}

fn calc_distance(pose: &Isometry<f32, UnitComplex<f32>, 2>, target_point: Point2<f32>) -> f32 {
    fn distance(point1: Point2<f32>, point2: Point2<f32>) -> f32 {
        ((point1.x - point2.x).powi(2) + (point1.y - point2.y).powi(2)).sqrt()
    }

    let robot_point = pose.translation.vector.into();

    distance(robot_point, target_point)
}

fn setup_path_visualizer(dbg: DebugContext) {
    dbg.log_with_cycle(
        "field/path",
        Cycle::default(),
        &rerun::LineStrips3D::update_fields()
            .with_colors([(66, 135, 245)])
            .with_radii([2.0]),
    );
}

fn log_planned_path(
    dbg: DebugContext,
    cycle: Res<Cycle>,
    robot_pose: Res<RobotPose>,
    mut step_planner: ResMut<StepPlanner>,
) {
    let path = step_planner.calc_path(&robot_pose);

    if let Some((path, _)) = path {
        dbg.log_with_cycle(
            "field/path",
            *cycle,
            &rerun::LineStrips3D::update_fields()
                .with_radii([0.02])
                .with_strips([LineStrip3D::from_iter(
                    path.iter().map(|point| (point.x, point.y, 0.05)),
                )]),
        );
    } else {
        dbg.log_with_cycle(
            "field/path",
            *cycle,
            &rerun::LineStrips3D::update_fields().with_strips(std::iter::empty::<LineStrip3D>()),
        );
    }
}

fn setup_dynamic_obstacle_logging(dbg: DebugContext) {
    dbg.log_static(
        "localization/pose/obstacles",
        &rerun::Capsules3D::update_fields()
            .with_colors([(204, 51, 51)])
            .with_lengths([0.5]),
    );
}

fn log_dynamic_obstacles(dbg: DebugContext, step_planner: Res<StepPlanner>, cycle: Res<Cycle>) {
    dbg.log_with_cycle(
        "localization/pose/obstacles",
        *cycle,
        &rerun::Capsules3D::update_fields()
            .with_translations(
                step_planner
                    .dynamic_obstacles
                    .iter()
                    .map(|obs| (obs.obs.x.0, obs.obs.y.0, 0.0)),
            )
            .with_radii(
                step_planner
                    .dynamic_obstacles
                    .iter()
                    .map(|obs| obs.obs.radius.0),
            ),
    );
}
