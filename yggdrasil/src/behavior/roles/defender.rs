use nalgebra::Point2;

use crate::{
    behavior::{
        behaviors::{Observe, WalkTo},
        engine::{BehaviorKind, Context, Control, Role},
    },
    motion::step_planner::Target,
};

/// The [`Defender`] role is held by any robot that does not see the ball.
/// It's job is to observe it's set position depending on player number.
#[derive(Debug)]
pub struct Defender;

impl Role for Defender {
    fn transition_behavior(&mut self, context: Context, control: &mut Control) -> BehaviorKind {
        let set_robot_position = context
            .layout_config
            .set_positions
            .player(context.player_config.player_number);

        let set_position = set_robot_position.isometry.translation.vector;
        let set_point = Point2::new(set_position.x, set_position.y);
        let defend_target = Target {
            position: set_point,
            rotation: Some(set_robot_position.isometry.rotation),
        };

        if (control.step_planner.has_target() && control.step_planner.reached_target())
            || (context.pose.distance_to(&set_point) < 0.4
                && (context.pose.world_rotation() - set_robot_position.isometry.rotation.angle())
                    .abs()
                    < 0.2)
        {
            if let BehaviorKind::Observe(observe) = context.current_behavior {
                return BehaviorKind::Observe(observe);
            }

            return BehaviorKind::Observe(Observe::with_turning(-0.4));
        }

        BehaviorKind::WalkTo(WalkTo {
            target: defend_target,
        })
    }
}
