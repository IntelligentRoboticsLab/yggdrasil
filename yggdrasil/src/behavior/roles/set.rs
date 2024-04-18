use crate::{
    behavior::{
        behaviors::StandingLookAt,
        engine::{BehaviorKind, Context, Role},
    },
    config::layout::WorldPosition,
    walk::engine::WalkingEngine,
};

pub struct Set;

impl Role for Set {
    fn transition_behavior(
        &mut self,
        _context: Context,
        _current_behavior: &mut BehaviorKind,
        _walking_engine: &mut WalkingEngine,
    ) -> BehaviorKind {
        let mut target = WorldPosition::new(0.0, 0.0);
        if false {
            // TODO: If there is a ball, look at the ball
            target = WorldPosition::new(20.0, 0.0);
        }

        BehaviorKind::StandingLookAt(StandingLookAt { target })
    }
}
