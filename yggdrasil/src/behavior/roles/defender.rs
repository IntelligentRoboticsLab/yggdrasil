use nalgebra::{Point2, UnitComplex};

use crate::{
    behavior::{
        behaviors::{Observe, WalkTo},
        engine::{BehaviorKind, Context, Control, Role},
    },
    motion::step_planner::Target,
};

#[derive(Debug)]
pub struct Defender;

impl Role for Defender {
    fn transition_behavior(&mut self, context: Context, _control: &mut Control) -> BehaviorKind {
        let set_robot_position = context
            .layout_config
            .set_positions
            .player(context.player_config.player_number);

        let set_position = set_robot_position.isometry.translation.vector;

        if context.pose.distance_to(&set_position.into()) < 0.1 {
            if let BehaviorKind::Observe(observe) = context.current_behavior {
                return BehaviorKind::Observe(observe.clone());
            }

            return BehaviorKind::Observe(Observe::default());
        }

        BehaviorKind::WalkTo(WalkTo {
            target: Target {
                position: Point2::new(set_position.x, set_position.y),
                rotation: Some(set_robot_position.isometry.rotation),
            },
        })
    }
}
