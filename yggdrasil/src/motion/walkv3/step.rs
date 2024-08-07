/// A step that can be taken by the walking engine.
pub struct Step {
    /// The forward component of the step, in meters.
    pub forward: f32,
    /// The left component of the step, in meters.
    pub left: f32,
    /// The turn component of the step, in radians.
    pub turn: f32,
}

impl Step {
    /// Create a new step with the given forward, left, and turn components.
    pub fn new(forward: f32, left: f32, turn: f32) -> Self {
        Self {
            forward,
            left,
            turn,
        }
    }

    /// Restrict the step to a certain range.
    pub fn clamp(mut self, min: Self, max: Step) -> Self {
        self.forward = self.forward.clamp(min.forward, max.forward);
        self.left = self.left.clamp(min.left, max.left);
        self.turn = self.turn.clamp(min.turn, max.turn);

        self
    }
}
