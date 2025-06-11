use bevy::prelude::*;
use nalgebra as na;

use crate::localization::RobotPose;

use super::{path_finding, step_planner::StepPlanner};

pub(super) struct ObstacleAvoidancePlugin;

impl Plugin for ObstacleAvoidancePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ObstacleAvoidance>()
            .add_systems(Update, plan_around_obstacles);
    }
}

#[derive(Default, Resource)]
pub struct ObstacleAvoidance(bool);

#[derive(Component)]
pub struct Obstacle {
    position: na::Point2<f32>,
    radius: f32,
}

impl From<&Obstacle> for path_finding::Obstacle {
    fn from(value: &Obstacle) -> Self {
        Self::new(value.position.x, value.position.y, value.radius)
    }
}

fn plan_around_obstacles(
    mut planner: ResMut<StepPlanner>,
    avoidance: Res<ObstacleAvoidance>,
    pose: Res<RobotPose>,
    obstacles: Query<&Obstacle>,
) {
    if !avoidance.0 {
        return;
    }

    if let Some(goal) = planner.waypoints.last().cloned() {
        let obstacles: Vec<path_finding::Obstacle> = obstacles.iter().map(|x| x.into()).collect();
        match path_finding::find_path(pose.world_position(), goal, &obstacles) {
            Some((waypoints, _)) => planner.waypoints = waypoints,
            None => planner.waypoints = Vec::new(),
        }
    }
}
