use crate::behavior::{
    behaviors::{Initial, Observe, Passive},
    engine::{BehaviorKind, Context, Role},
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
                } else if context.head_buttons.middle.is_pressed() {
                    BehaviorKind::Observe(Observe::default())
                } else {
                    BehaviorKind::Passive(Passive::default())
                }
            }
            BehaviorKind::Initial(state) => BehaviorKind::Initial(*state),
            BehaviorKind::Observe(state) => BehaviorKind::Observe(*state),
            BehaviorKind::Penalized(state) => BehaviorKind::Penalized(*state),
        }
    }
}
