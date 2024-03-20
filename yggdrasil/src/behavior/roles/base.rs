use crate::behavior::{
    behaviors::{Initial, Passive},
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
        if context.chest_button.state.is_pressed() {
            // If chest button is pressed transition to initial behavior.
            match current_behavior {
                BehaviorKind::Passive(_) => BehaviorKind::Initial(Initial),
                BehaviorKind::Initial(state) => BehaviorKind::Initial(*state),
            }
        } else {
            BehaviorKind::Passive(Passive)
        }
    }
}
