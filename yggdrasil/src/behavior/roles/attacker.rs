use std::sync::{Arc, RwLock};

use nalgebra::Point2;

use crate::{
    behavior::{
        behaviors::{PolicyModel, Walk},
        engine::{BehaviorKind, Context, Control, Role},
    },
    core::ml::MlTask,
    motion::step_planner::Target,
};

pub struct Attacker;

impl Role for Attacker {
    fn transition_behavior(&mut self, context: Context, _control: &mut Control) -> BehaviorKind {
        BehaviorKind::Walk(Walk {
            target: Target {
                position: context.ball_position.unwrap_or(Point2::new(0.0, 0.0)),
                rotation: None,
            },
        })
    }
}
