use crate::{
    behavior::{
        behaviors::Walk,
        engine::{BehaviorKind, Context, Role},
    },
    motion::keyframe::KeyframeExecutor,
    motion::step_planner::StepPlanner,
    motion::walk::engine::{Step, WalkingEngine},
};

pub struct Attacker;

impl Role for Attacker {
    fn transition_behavior(
        &mut self,
        _context: Context,
        _current_behavior: &mut BehaviorKind,
        _: &mut WalkingEngine,
        _: &mut KeyframeExecutor,
        _step_planner: &mut StepPlanner,
    ) -> BehaviorKind {
        BehaviorKind::Walk(Walk {
            step: Step {
                forward: 0.04,
                left: 0.0,
                turn: 0.0,
            },
        })
    }
}
