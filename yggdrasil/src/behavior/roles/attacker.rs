use nalgebra::Point2;

use crate::behavior::{
    behaviors::Walk,
    engine::{BehaviorKind, Context, Control, Role},
};

pub struct Attacker;

impl Role for Attacker {
    fn transition_behavior(&mut self, context: Context, _control: &mut Control) -> BehaviorKind {
        let target = context.ball_position.unwrap_or(Point2::new(0.0, 0.0));

        BehaviorKind::Walk(Walk { target })
    }
}
