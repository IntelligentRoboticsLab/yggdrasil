use nalgebra::{ComplexField, Isometry2, Point2, Translation2, UnitComplex};

use crate::{
    behavior::{
        behaviors::{Observe, Walk, WalkTo},
        engine::{BehaviorKind, Context, Role},
    },
    motion::step_planner::StepPlanner,
    config::layout::WorldPosition,
    motion::{odometry::isometry_to_absolute, step_planning::StepPlanner},
    walk::engine::{Step, WalkingEngine},
};

pub struct Attacker;

impl Role for Attacker {
    fn transition_behavior(
        &mut self,
        context: Context,
        current_behavior: &mut BehaviorKind,
        _walking_engine: &mut WalkingEngine,
        step_planner: &mut StepPlanner,
    ) -> BehaviorKind {
        if context.ball_position.balls.len() >= 1 {
            let ball_position = context.ball_position.balls[0].clone();
            let robot_position = context.robot_position;

            let pos = context
                .layout_config
                .initial_positions
                .player(context.player_config.player_number);
            let target = isometry_to_absolute(
                Isometry2::from_parts(
                    Translation2::from(ball_position.robot_to_ball),
                    UnitComplex::identity(),
                ),
                pos,
            );

            return BehaviorKind::WalkTo(WalkTo {
                target: target.translation.vector.into(),
            });
        }

        if let BehaviorKind::Observe(observe) = current_behavior {
            BehaviorKind::Observe(*observe)
        } else {
            BehaviorKind::Observe(Observe::default())
        }
    }
}
