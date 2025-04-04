use std::sync::{Arc, RwLock};

use re_control_comms::protocol::RobotMessage;

#[derive(Default)]
pub struct TrackedState<T> {
    current: T,
    original: T,
}

impl<T> TrackedState<T>
where
    T: Clone,
{
    pub fn current(&self) -> &T {
        &self.current
    }

    pub fn current_mut(&mut self) -> &mut T {
        &mut self.current
    }

    pub fn new_state(&mut self, state: T) {
        self.current = state.clone();
        self.original = state;
    }

    pub fn restore_original(&mut self) {
        self.current = self.original.clone();
    }
}

pub trait HandleState {
    fn handle_message(&mut self, message: &RobotMessage);

    fn reset(&mut self)
    where
        Self: Default,
    {
        std::mem::take(self);
    }
}

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
