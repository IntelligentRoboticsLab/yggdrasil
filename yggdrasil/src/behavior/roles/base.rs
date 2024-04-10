use crate::{
    behavior::{
        behaviors::{Initial, Observe, StartUp, Unstiff},
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
                if walking_engine.hip_height < 0.1 {
                    return BehaviorKind::Unstiff(Unstiff);
                }
                BehaviorKind::StartUp(StartUp)
            }
            BehaviorKind::Unstiff(_) => {
                // If chest button is pressed transition to initial behavior.
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
            BehaviorKind::Observe(state) => BehaviorKind::Observe(*state),
            BehaviorKind::Penalized(state) => BehaviorKind::Penalized(*state),
        }
    }
}
