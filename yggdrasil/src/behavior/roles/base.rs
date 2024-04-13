use crate::{
    behavior::{
        behaviors::{Initial, Observe, Penalized, StartUp, Unstiff},
        engine::{BehaviorKind, Context, Role},
    },
    walk::engine::WalkingEngine,
};

// This role is a placeholder that implements general behaviors until we implement
// specific roles.
pub struct Base;

impl Role for Base {
    fn transition_behavior(
        &mut self,
        context: Context,
        current_behavior: &mut BehaviorKind,
        walking_engine: &mut WalkingEngine,
    ) -> BehaviorKind {
        match current_behavior {
            BehaviorKind::StartUp(_) => {
                if walking_engine.is_sitting() {
                    return BehaviorKind::Unstiff(Unstiff);
                }
                if context.chest_button.state.is_tapped() {
                    return BehaviorKind::Initial(Initial);
                }
                BehaviorKind::StartUp(StartUp)
            }
            BehaviorKind::Unstiff(_) => {
                if context.chest_button.state.is_tapped() {
                    return BehaviorKind::Initial(Initial);
                }

                BehaviorKind::Unstiff(Unstiff)
            }
            BehaviorKind::Initial(state) => {
                if context.chest_button.state.is_tapped() {
                    return BehaviorKind::Observe(Observe::default());
                }

                BehaviorKind::Initial(*state)
            }
            BehaviorKind::Observe(state) => {
                if context.chest_button.state.is_tapped() {
                    return BehaviorKind::Penalized(Penalized);
                }
                BehaviorKind::Observe(*state)
            }
            BehaviorKind::Penalized(state) => {
                if context.chest_button.state.is_tapped() {
                    return BehaviorKind::Observe(Observe::default());
                }
                BehaviorKind::Penalized(*state)
            }
        }
    }
}
