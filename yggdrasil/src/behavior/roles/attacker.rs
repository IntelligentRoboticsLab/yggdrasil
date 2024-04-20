use crate::{
    behavior::{
        behaviors::EnergyEfficientStand,
        engine::{BehaviorKind, Context, Role},
    },
    motion::{motion_manager::MotionManager, step_planner::StepPlanner},
    walk::engine::WalkingEngine,
};

pub struct Attacker;

impl Role for Attacker {
    fn transition_behavior(
        &mut self,
        _context: Context,
        _current_behavior: &mut BehaviorKind,
        _: &mut WalkingEngine,
        _: &mut MotionManager,
        _step_planner: &mut StepPlanner,
    ) -> BehaviorKind {
        // BehaviorKind::Walk(Walk {
        //     step: Step {
        //         forward: 0.04,
        //         left: 0.0,
        //         turn: 0.0,
        //     },
        // })
        match _current_behavior {
            BehaviorKind::EnergyEfficientStand(state) => BehaviorKind::EnergyEfficientStand(*state),
            _ => BehaviorKind::EnergyEfficientStand(EnergyEfficientStand::default()),
        }
    }
}
