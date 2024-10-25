use nalgebra::{Point2, UnitComplex};

use crate::{
    behavior::{
        behaviors::{Observe, WalkTo},
        engine::{BehaviorKind, Context, Control, Role},
    },
    motion::step_planner::Target,
};

/// The Keeper role is held by a single robot at a time, usually player number 1.
/// It's job is to prevent the ball from entering the goal, which it does by staying in the goal area.
#[derive(Debug)]
pub struct Keeper;

impl Role for Keeper {
    fn transition_behavior(&mut self, context: Context, control: &mut Control) -> BehaviorKind {
        let keeper_target = Target {
            position: Point2::new(-context.layout_config.field.length / 2., 0.),
            rotation: Some(UnitComplex::<f32>::from_angle(0.0)),
        };
        if !control.step_planner.has_target() {
            return BehaviorKind::WalkTo(WalkTo {
                target: keeper_target,
            });
        }
        if control.step_planner.reached_target() {
            if let BehaviorKind::Observe(observe) = context.current_behavior {
                return BehaviorKind::Observe(observe);
            }

            return BehaviorKind::Observe(Observe::default());
        }

        BehaviorKind::WalkTo(WalkTo {
            target: keeper_target,
        })
    }
}
