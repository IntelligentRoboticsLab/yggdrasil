//! Higher-level pathfinding capabilities.

use nalgebra as na;
use bevy::prelude::*;

use crate::{core::debug::DebugContext, localization::RobotPose};

use super::{finding::{Path, Pathfinding, PathfindingSettings}, obstacles::Colliders};

pub fn update_path(
    mut path: ResMut<Path>,
    pose: Res<RobotPose>,
    colliders: Res<Colliders>,
) {
    let pathfinding = Pathfinding {
        start: pose.inner.into(),
        goal: na::Isometry2::new(na::vector![3., -3.], 0.).into(),
        colliders: &colliders,
        settings: PathfindingSettings {
            ccw_ease_in: 1.,
            cw_ease_in: 1.,
            ccw_ease_out: 1.,
            cw_ease_out: 1.,
        },
    };

    if let Some((new, _)) = pathfinding.path() {
        *path = new;
    }
}

pub fn visualize_path(dbg: DebugContext, path: Res<Path>) {
    dbg.log(
        "pathfinding/path",
        &rerun::LineStrips3D::new([
            path
                .0
                .iter()
                .map(|s| s.vertices(64.))
                .flatten()
                .map(|p| [p.x, p.y, 0.10])
        ]),
    );
}
