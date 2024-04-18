use nalgebra::{ComplexField, Point2};

use crate::{
    behavior::{
        behaviors::{Walk, WalkTo},
        engine::{BehaviorKind, Context, Role},
    },
    motion::step_planner::StepPlanner,
    config::layout::WorldPosition,
    motion::step_planning::StepPlanner,
    walk::engine::{Step, WalkingEngine},
};

pub struct Attacker;

impl Role for Attacker {
    fn transition_behavior(
        &mut self,
        context: Context,
        _current_behavior: &mut BehaviorKind,
        _walking_engine: &mut WalkingEngine,
        _step_planner: &mut StepPlanner,
    ) -> BehaviorKind {
        let ball_position = *context.ball_position;
        let robot_position = context.robot_position;
        let goal_position = WorldPosition::new(40.0, 0.0);

        // if distance to ball is less than 1.0, kick the ball

        BehaviorKind::WalkTo(WalkTo {
            target: ball_position,
        })
    }
}
