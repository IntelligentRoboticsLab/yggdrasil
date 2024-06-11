use crate::{
    behavior::engine::{Behavior, Context, Control},
    nao::manager::Priority,
};

/// Behavior used for preventing damage when the robot is in a falling state.
/// This behavior can only be exited from once the robot is lying down.
///
/// # Notes
/// - Currently, the damage prevention is still quite rudimentary, only
///   unstiffing the joints of the robot and making the head stiff.
///   In future this will be accompanied by an appropriate safe falling
///   position.
/// - If the robot incorrectly assumes it is in a falling state yet
///   will not be lying down, the robot will kinda get "soft-locked".
///   Only by unstiffing the robot will it return to normal.
///   This should not be the case often however, once the falling filter
///   is more advanced.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct CatchFall;

impl Behavior for CatchFall {
    fn execute(&mut self, _: Context, control: &mut Control) {
        control.nao_manager.unstiff_legs(Priority::Critical);
        control.nao_manager.unstiff_arms(Priority::Critical);
        control.nao_manager.unstiff_head(Priority::Critical);
    }
}
