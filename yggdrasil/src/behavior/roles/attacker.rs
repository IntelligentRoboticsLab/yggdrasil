use nalgebra::{ComplexField, Point2};

use crate::{
    behavior::{
        behaviors::{AlignWith, Walk, WalkTo},
        engine::{BehaviorKind, Context, Role},
    },
    motion::step_planner::StepPlanner,
    config::layout::WorldPosition,
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

        if goal_position.y().abs() > 0.1 {
            // We are alligned with the goal, kick the ball by walking forward
            BehaviorKind::AlignWith(AlignWith {
                target: goal_position,
                center: WorldPosition::new(ball_position.x, ball_position.y),
            })
        } else {
            BehaviorKind::WalkTo(WalkTo {
                target: ball_position,
            })
        }
    }
}
