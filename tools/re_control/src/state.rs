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
