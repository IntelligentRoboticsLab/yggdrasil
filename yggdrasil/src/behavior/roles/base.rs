use crate::behavior::{
    behaviors::{Initial, Passive, Test},
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
                if context.chest_button.state.is_tapped() {
                    BehaviorKind::Initial(Initial)
                } else {
                    BehaviorKind::Passive(Passive)
                }
            }
            BehaviorKind::Initial(state) => {
                if context.chest_button.state.is_tapped() {
                    BehaviorKind::Test(Test)
                } else {
                    BehaviorKind::Initial(*state)
                }
            }
            BehaviorKind::Observe(state) => BehaviorKind::Observe(*state),
            BehaviorKind::Penalized(_) => BehaviorKind::Initial(Initial),
            BehaviorKind::Test(_) => BehaviorKind::Test(Test),
        }
    }
}
