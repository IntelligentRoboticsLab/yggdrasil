use crate::behavior::{
    behaviors::Example,
    engine::{BehaviorKind, Context, Role},
};

pub struct Keeper;

impl Role for Keeper {
    fn transition_behavior(
        &mut self,
        context: Context,
        current_behavior: &mut BehaviorKind,
    ) -> BehaviorKind {
        if context.head_buttons.middle.is_pressed() {
            match current_behavior {
                BehaviorKind::Example(state) => BehaviorKind::Example(*state),
                _ => BehaviorKind::Example(Example::default()),
            }
        } else {
            BehaviorKind::default()
        }
    }
}
