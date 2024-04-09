use crate::{
    behavior::{
        behaviors::{Initial, Observe, Passive, Penalized},
        engine::{BehaviorKind, Context, Role},
    },
    primary_state::PrimaryState,
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
        if *context.primary_state == PrimaryState::Penalized
            && !matches!(current_behavior, BehaviorKind::Passive(_))
        {
            return BehaviorKind::Penalized(Penalized);
        }

        match current_behavior {
            BehaviorKind::Passive(_) => {
                // If chest button is pressed transition to initial behavior.
                if context.chest_button.state.is_pressed() {
                    BehaviorKind::Initial(Initial)
                } else if context.head_buttons.middle.is_pressed() {
                    BehaviorKind::Observe(Observe::default())
                } else {
                    BehaviorKind::Passive(Passive)
                }
            }
            BehaviorKind::Initial(state) => BehaviorKind::Initial(*state),
            BehaviorKind::Observe(state) => BehaviorKind::Observe(*state),
            BehaviorKind::Penalized(_) => BehaviorKind::Initial(Initial),
        }
    }
}
