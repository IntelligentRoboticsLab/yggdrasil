use super::walking_engine::step::Step;
use crate::{core::debug::DebugContext, localization::RobotPose, nao::Cycle};
use bevy::prelude::*;
use nalgebra::{Isometry, Point2, UnitComplex, Vector2, Vector3};
use rerun::{FillMode, LineStrip3D};
use std::{f32::consts::PI, time::Instant};

use crate::motion::rrt_path_planner::{
    Obstacle, assign_headings_from_xy, plan_se2_with_obstacles, wrap_to_pi,
};

const TURN_SPEED: f32 = 0.2;
const WALK_SPEED: f32 = 0.045;

const REPLAN_POS_TOL: f32 = 0.10; // m robot moved from last plan
const REPLAN_YAW_TOL: f32 = 0.35; // rad (~20°)
const REPLAN_PATH_DEVIATION: f32 = 0.15; // m from path polyline
const REPLAN_MAX_AGE: f32 = 2.0; // sec

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

    se2_path: Vec<Vector3<f32>>, // latest planned path (start→goal)
    se2_path_version: u64,       // bump when replanning (debug only)

    target_version: u64,
    static_obs_version: u64,
    dynamic_obs_version: u64,

    last_plan_target_version: u64,
    last_plan_static_obs_version: u64,
    last_plan_dynamic_obs_version: u64,
    last_plan_robot_xy: Point2<f32>,
    last_plan_robot_yaw: f32,
    last_plan_time: Instant,
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
                Obstacle::new(-4.0, -1.1, 0.75),
            ],
            dynamic_obstacles: vec![],
            se2_path: Vec::new(),
            se2_path_version: 0,
            last_plan_target_version: u64::MAX, // force initial plan
            last_plan_static_obs_version: u64::MAX,
            last_plan_dynamic_obs_version: u64::MAX,
            last_plan_robot_xy: Point2::new(f32::NAN, f32::NAN),
            last_plan_robot_yaw: f32::NAN,
            last_plan_time: Instant::now(),
            static_obs_version: 0,
            dynamic_obs_version: 0,
            target_version: 0,
        }
    }
}

impl StepPlanner {
    pub fn set_absolute_target(&mut self, target: Target) {
        self.clear_target();
        self.target = Some(target);
        self.target_version = self.target_version.wrapping_add(1);
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
        self.target_version = self.target_version.wrapping_add(1);
    }

    #[must_use]
    pub fn current_absolute_target(&self) -> Option<&Target> {
        self.target.as_ref()
    }

    /// Dynamic obstacles need to be added in relative coordinates.
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

    /// Retrieves all currently relevant objects, in absolute coordinates.
    fn get_all_obstacles(&mut self, robot_pose: &RobotPose) -> Vec<Obstacle> {
        let all_dynamic_obstacles = self.collect_and_gc_dynamic_obstacles();

        let abs_dynamic_obstacles: Vec<_> = all_dynamic_obstacles
            .iter()
            .map(|obs| {
                let abs_pos = robot_pose.robot_to_world(&Point2::new(obs.x.0, obs.y.0));
                Obstacle::new(abs_pos.x, abs_pos.y, obs.radius.0)
            })
            .collect();

        let mut all_obstacles = self.static_obstacles.clone();
        all_obstacles.extend_from_slice(&abs_dynamic_obstacles);

        all_obstacles
    }

    /// Plan a full SE(2) path (x,y,θ) from current robot pose to current target.
    ///
    /// Returns: Some((path_xy, total_dist)) for legacy API; also populates `self.se2_path`.
    /// `path_xy` is just the XY projection of the SE(2) path (for logging & old callers).
    /// Plan (or reuse cached) SE(2) path. If `force` false, uses change detection.
    fn calc_path(&mut self, robot_pose: &RobotPose) -> Option<(Vec<Point2<f32>>, f32)> {
        if !self.should_replan(robot_pose) {
            // Return legacy view of cached path.
            let mut path_xy = Vec::with_capacity(self.se2_path.len());
            let mut dist = 0.0;
            for (i, s) in self.se2_path.iter().enumerate() {
                let p = Point2::new(s[0], s[1]);
                if i > 0 {
                    let prev = &self.se2_path[i - 1];
                    let dx = s[0] - prev[0];
                    let dy = s[1] - prev[1];
                    dist += (dx * dx + dy * dy).sqrt();
                }
                path_xy.push(p);
            }
            return Some((path_xy, dist));
        }

        tracing::info!("recomputing path!");

        // --- FALL THROUGH: recompute ---
        let target = self.target?;
        let start_xy = robot_pose.world_position();
        let start_yaw = wrap_to_pi(robot_pose.world_rotation());
        let goal_yaw = if let Some(rot) = target.rotation {
            wrap_to_pi(rot.angle())
        } else {
            wrap_to_pi((target.position.y - start_xy.y).atan2(target.position.x - start_xy.x))
        };
        let all_obstacles = self.get_all_obstacles(robot_pose);

        const STEP_SIZE: f32 = 0.05;
        const SEARCH_RADIUS: f32 = 0.20;
        const MAX_ITERS: usize = 3000;
        const GOAL_BIAS: f32 = 0.10;

        let se2_path = plan_se2_with_obstacles(
            start_xy,
            start_yaw,
            target.position,
            goal_yaw,
            all_obstacles,
            STEP_SIZE,
            SEARCH_RADIUS,
            MAX_ITERS,
            GOAL_BIAS,
            None,
        )?;

        // cache full path
        self.se2_path = se2_path;
        self.se2_path_version = self.se2_path_version.wrapping_add(1);

        // snapshot versions & pose
        self.last_plan_target_version = self.target_version;
        self.last_plan_static_obs_version = self.static_obs_version;
        self.last_plan_dynamic_obs_version = self.dynamic_obs_version;
        self.last_plan_robot_xy = start_xy;
        self.last_plan_robot_yaw = start_yaw;
        self.last_plan_time = Instant::now();

        // produce XY + dist
        let mut path_xy = Vec::with_capacity(self.se2_path.len());
        let mut dist = 0.0;
        for (i, s) in self.se2_path.iter().enumerate() {
            let p = Point2::new(s[0], s[1]);
            if i > 0 {
                let prev = &self.se2_path[i - 1];
                let dx = s[0] - prev[0];
                let dy = s[1] - prev[1];
                dist += (dx * dx + dy * dy).sqrt();
            }
            path_xy.push(p);
        }
        Some((path_xy, dist))
    }

    fn should_replan(&self, robot_pose: &RobotPose) -> bool {
        // --- 1. No cached path? ---
        if self.se2_path.is_empty() {
            return true;
        }

        // --- 2. Version bumps? ---
        if self.target_version != self.last_plan_target_version
            || self.static_obs_version != self.last_plan_static_obs_version
            || self.dynamic_obs_version != self.last_plan_dynamic_obs_version
        {
            return true;
        }

        // --- 3. Robot moved a lot since last plan origin? ---
        let curr_xy = robot_pose.world_position();
        let curr_yaw = wrap_to_pi(robot_pose.world_rotation());
        let dx = curr_xy.x - self.last_plan_robot_xy.x;
        let dy = curr_xy.y - self.last_plan_robot_xy.y;
        if (dx * dx + dy * dy).sqrt() > REPLAN_POS_TOL {
            return true;
        }
        if (wrap_to_pi(curr_yaw - self.last_plan_robot_yaw)).abs() > REPLAN_YAW_TOL {
            return true;
        }

        // --- 4. Off the path? ---
        if self.dist_to_current_path(curr_xy) > REPLAN_PATH_DEVIATION {
            return true;
        }

        // --- 5. Stale? ---
        if self.last_plan_time.elapsed().as_secs_f32() > REPLAN_MAX_AGE {
            return true;
        }

        false
    }

    /// Distance from current XY to nearest segment in cached se2_path.
    fn dist_to_current_path(&self, p: Point2<f32>) -> f32 {
        if self.se2_path.len() < 2 {
            return f32::INFINITY;
        }
        let mut best = f32::INFINITY;
        for w in self.se2_path.windows(2) {
            let ax = w[0][0];
            let ay = w[0][1];
            let bx = w[1][0];
            let by = w[1][1];
            // point-segment dist
            let abx = bx - ax;
            let aby = by - ay;
            let apx = p.x - ax;
            let apy = p.y - ay;
            let ab2 = abx * abx + aby * aby;
            let t = if ab2 > 0.0 {
                (apx * abx + apy * aby) / ab2
            } else {
                0.0
            }
            .clamp(0.0, 1.0);
            let cx = ax + t * abx;
            let cy = ay + t * aby;
            let dx = p.x - cx;
            let dy = p.y - cy;
            let d2 = dx * dx + dy * dy;
            if d2 < best {
                best = d2;
            }
        }
        best.sqrt()
    }

    fn plan_step_from_se2_path(robot_pose: &RobotPose, se2_path: &[Vector3<f32>]) -> Option<Step> {
        if se2_path.len() < 2 {
            return None;
        }

        // Current robot pose
        let curr_xy = robot_pose.world_position();
        let curr_yaw = wrap_to_pi(robot_pose.world_rotation());

        // Find the segment we should be on: first node ahead beyond small tolerance.
        // (Cheap linear scan; path is short.)
        const ADVANCE_TOL: f32 = 0.07; // m
        let mut next_idx = 1;
        for i in 1..se2_path.len() {
            let dx = se2_path[i][0] - curr_xy.x;
            let dy = se2_path[i][1] - curr_xy.y;
            if (dx * dx + dy * dy).sqrt() > ADVANCE_TOL {
                next_idx = i;
                break;
            }
        }
        let target_state = se2_path[next_idx];
        let target_xy = Point2::new(target_state[0], target_state[1]);
        let target_yaw = target_state[2];

        // Body‑frame delta XY
        // RobotPose gives world->robot transform via inverse_transform_point.
        let rel = robot_pose.inner.inverse_transform_point(&target_xy);
        let rel_vec = Vector2::new(rel.x, rel.y);

        // Heading error (in world frame)
        let yaw_err = wrap_to_pi(target_yaw - curr_yaw);

        // Distance forward
        let dist = rel_vec.norm();

        // --- Simple control policy ---
        // Turn gain: scale TURN_SPEED by normalized yaw error (|err| ≤ π).
        let turn_cmd = (yaw_err / PI).clamp(-1.0, 1.0) * TURN_SPEED;

        // Forward gain: fade out if facing away; fade in with cos of error
        let facing_gain = yaw_err.cos().max(0.0); // don't walk backward
        let forward_cmd = (dist.min(0.20) / 0.20) * WALK_SPEED * facing_gain;

        let left_cmd = 0.0;
        // let left_cmd = rel_vec.y.signum() * WALK_SPEED * 0.5;

        // Final closeout: if final node and close
        if next_idx + 1 == se2_path.len() && dist < 0.05 && yaw_err.abs() < 0.2 {
            return None;
        }

        Some(Step {
            forward: forward_cmd,
            left: left_cmd,
            turn: turn_cmd,
        })
    }

    // shim old fns so external code compiles; you can delete once all callsites updated
    fn plan_translation(robot_pose: &RobotPose, path: &[Point2<f32>]) -> Option<Step> {
        // Fallback: treat dest heading as bearing
        let curr = robot_pose.world_position();
        let dest = path[1];
        let bearing = wrap_to_pi((dest.y - curr.y).atan2(dest.x - curr.x));
        let yaw_err = wrap_to_pi(bearing - robot_pose.world_rotation());

        let turn_cmd = (yaw_err / PI).clamp(-1.0, 1.0) * TURN_SPEED;
        let dist = ((dest.x - curr.x).powi(2) + (dest.y - curr.y).powi(2)).sqrt();
        let facing_gain = yaw_err.cos().max(0.0);
        let forward_cmd = (dist.min(0.20) / 0.20) * WALK_SPEED * facing_gain;

        if path.len() == 2 && dist < 0.05 && yaw_err.abs() < 0.2 {
            None
        } else {
            Some(Step {
                forward: forward_cmd,
                left: 0.0,
                turn: turn_cmd,
            })
        }
    }

    fn plan_rotation(robot_pose: &RobotPose, target_rotation: UnitComplex<f32>) -> Option<Step> {
        let curr_yaw = wrap_to_pi(robot_pose.world_rotation());
        let goal_yaw = wrap_to_pi(target_rotation.angle());
        let yaw_err = wrap_to_pi(goal_yaw - curr_yaw);
        if yaw_err.abs() < 0.2 {
            None
        } else {
            let turn_cmd = (yaw_err / PI).clamp(-1.0, 1.0) * TURN_SPEED;
            Some(Step {
                forward: 0.0,
                left: 0.0,
                turn: turn_cmd,
            })
        }
    }

    pub fn plan(&mut self, robot_pose: &RobotPose) -> Option<Step> {
        let target = self.target?;
        let (path, _total_walking_distance) = self.calc_path(robot_pose)?;

        if let step @ Some(_) = Self::plan_translation(robot_pose, &path) {
            if !self.reached_translation_target {
                return step;
            }
        }

        self.reached_translation_target = true;

        if let Some(rotation) = target.rotation.as_ref() {
            if let step @ Some(_) = Self::plan_rotation(robot_pose, *rotation) {
                return step;
            }
        }

        self.reached_rotation_target = true;

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
            .with_radii([0.01]),
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
            &rerun::LineStrips3D::update_fields().with_strips([LineStrip3D::from_iter(
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
        &rerun::Ellipsoids3D::update_fields()
            .with_colors([(69, 255, 249)])
            .with_fill_mode(FillMode::Solid),
    );
}

fn log_dynamic_obstacles(dbg: DebugContext, step_planner: Res<StepPlanner>, cycle: Res<Cycle>) {
    let centers = step_planner
        .dynamic_obstacles
        .iter()
        .map(|obs| (obs.obs.x.0, obs.obs.y.0, -0.28))
        .collect::<Vec<_>>();

    let half_sizes = step_planner
        .dynamic_obstacles
        .iter()
        .map(|obs| (obs.obs.radius.0, obs.obs.radius.0, 0.4))
        .collect::<Vec<_>>();

    dbg.log_with_cycle(
        "localization/pose/obstacles",
        *cycle,
        &rerun::Ellipsoids3D::update_fields()
            .with_centers(centers)
            .with_half_sizes(half_sizes),
    );
}
