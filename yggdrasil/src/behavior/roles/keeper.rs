use crate::behavior::{behaviors::Example, engine::Transition, Behavior, Context};

pub struct Keeper;

impl Transition for Keeper {
    fn transition_behavior(
        &mut self,
        context: Context,
        current_behavior: &mut Behavior,
    ) -> Behavior {
        if context.head_buttons.middle.is_pressed() {
            match current_behavior {
                Behavior::Example(state) => Behavior::Example(*state),
                _ => Behavior::Example(Example::default()),
            }
        } else {
            Behavior::default()
        }
    }
}
