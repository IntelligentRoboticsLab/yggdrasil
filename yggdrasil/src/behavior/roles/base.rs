use crate::{
    behavior::{
        behaviors::{Initial, Observe, Passive, Standup},
        engine::{BehaviorKind, Context, Role},
    },
    filter::falling::FallState,
};

// This role is a placeholder that implements general behaviors until we implement
// specific roles.
pub struct Base;

impl Role for Base {
    fn transition_behavior(
        &mut self,
        context: Context,
        current_behavior: &mut BehaviorKind,
    ) -> BehaviorKind {
        match current_behavior {
            BehaviorKind::Passive(_) => {
                // If chest button is pressed transition to initial behavior.
                if context.chest_button.state.is_pressed() {
                    BehaviorKind::Initial(Initial)
                } else {
                    BehaviorKind::Passive(Passive)
                }
            }
            BehaviorKind::Initial(state) => BehaviorKind::Initial(*state),
            BehaviorKind::Observe(state) => BehaviorKind::Observe(*state),
            BehaviorKind::Penalized(_) => BehaviorKind::Initial(Initial),
            BehaviorKind::Standup(state) => match context.fall_state {
                FallState::Upright => BehaviorKind::Observe(Observe::default()),
                _ => BehaviorKind::Standup(*state),
            },
        }
    }
}
