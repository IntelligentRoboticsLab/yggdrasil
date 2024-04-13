use crate::{
    behavior::{
        behaviors::{Initial, Observe, Penalized, Unstiff},
        engine::BehaviorKind,
    },
    primary_state::PrimaryState,
    walk::engine::WalkingEngine,
};

pub fn transition_base(
    current_behavior: &BehaviorKind,
    walking_engine: &mut WalkingEngine,
    primary_state: &PrimaryState,
) -> Option<BehaviorKind> {
    if let BehaviorKind::StartUp(_) = current_behavior {
        if walking_engine.is_sitting() {
            return Some(BehaviorKind::Unstiff(Unstiff));
        }
    }
    match primary_state {
        PrimaryState::Unstiff => Some(BehaviorKind::Unstiff(Unstiff)),
        PrimaryState::Penalized => Some(BehaviorKind::Penalized(Penalized)),
        PrimaryState::Initial => Some(BehaviorKind::Initial(Initial)),
        PrimaryState::Ready => Some(BehaviorKind::Initial(Initial)),
        PrimaryState::Set => Some(BehaviorKind::Initial(Initial)),
        PrimaryState::Playing => Some(BehaviorKind::Observe(Observe::default())),
        PrimaryState::Finished => Some(BehaviorKind::Initial(Initial)),
        PrimaryState::Calibration => Some(BehaviorKind::Initial(Initial)),
    }
}
