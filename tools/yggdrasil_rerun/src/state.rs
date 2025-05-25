use std::sync::{Arc, RwLock};

use yggdrasil_rerun_comms::protocol::RobotMessage;

/// A wrapper around a type that keeps two version of a state: The current
/// state, and the original state. The current state can be overwritten by
/// the remembered original state.
#[derive(Default)]
pub struct TrackedState<T> {
    current: T,
    original: T,
}

impl<T> TrackedState<T>
where
    T: Clone,
{
    /// Gives the current state
    pub fn current(&self) -> &T {
        &self.current
    }

    /// A mutable reference to the current state
    pub fn current_mut(&mut self) -> &mut T {
        &mut self.current
    }

    /// Reset the original state (and the current state) to the new
    /// give state
    pub fn new_state(&mut self, state: T) {
        self.current = state.clone();
        self.original = state;
    }

    /// Overwrite the current state with the original state
    pub fn restore_original(&mut self) {
        self.current = self.original.clone();
    }
}

/// Trait to alter a certain state. The state is modified by [`RobotMessage`]
/// or by resetting it to the default of the implemented type
pub trait HandleState {
    fn handle_message(&mut self, message: &RobotMessage);

    fn reset(&mut self)
    where
        Self: Default,
    {
        std::mem::take(self);
    }
}

/// Trait that is similar to the [`HandleState`] trait but does not need mutability.
pub trait SharedHandleState {
    fn handle_message(&self, message: &RobotMessage);

    fn reset(&self)
    where
        Self: Default;
}

impl<T> SharedHandleState for Arc<RwLock<T>>
where
    T: HandleState + Default,
{
    fn handle_message(&self, message: &RobotMessage) {
        let mut locked_data = self.write().expect("failed to lock data");
        locked_data.handle_message(message);
    }

    fn reset(&self) {
        self.write().expect("failed to lock data").reset();
    }
}
