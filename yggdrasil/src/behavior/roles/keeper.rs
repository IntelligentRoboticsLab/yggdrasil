use nalgebra::{Point2, UnitComplex};

use crate::{
    behavior::{
        behaviors::{Observe, Walk},
        engine::{BehaviorKind, Context, Control, Role},
    },
    motion::step_planner::Target,
};

pub struct Keeper;

impl Role for Keeper {
    fn transition_behavior(&mut self, context: Context, control: &mut Control) -> BehaviorKind {
        let keeper_target = Target {
            position: Point2::new(-context.layout_config.field.length / 2., 0.),
            rotation: Some(UnitComplex::<f32>::from_angle(0.0)),
        };

        if control
            .step_planner
            .current_absolute_target()
            .is_some_and(|target| target == &keeper_target)
            || control.step_planner.reached_target()
        {
            if let BehaviorKind::Observe(observe) = context.current_behavior {
                return BehaviorKind::Observe(observe);
            } else {
                return BehaviorKind::Observe(Observe::default());
            };
        }

        BehaviorKind::Walk(Walk {
            target: keeper_target,
        })
    }
}
