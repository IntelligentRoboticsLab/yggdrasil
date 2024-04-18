use nalgebra::Point2;

use crate::{
    behavior::engine::{Behavior, Context},
    config::layout::{RobotPosition, WorldPosition},
    nao::manager::NaoManager,
    walk::engine::{Step, WalkingEngine},
};

#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct AlignWith {
    pub target: WorldPosition,
    pub center: WorldPosition,
}

impl Behavior for AlignWith {
    fn execute(
        &mut self,
        context: Context,
        _nao_manager: &mut NaoManager,
        walking_engine: &mut WalkingEngine,
    ) {
        // x is infront, y is to the left
        let target_relative = self.target;

        let mut step = Step::default();

        // we have an object which is at center and a target position to align ourselves which is 0,0 as the coordinates are relative to us
        // we want to side step along a circle around the center to align ourselves with the target
        // we can do this by calculating the angle between the center and the target and then walk around the center to align ourselves with the target

        // First the vector from the center to the target
        let target_vector = Point2::new(target_relative.x(), target_relative.y());

        // secondly the vector from the center to us
        let center_vector = Point2::new(self.center.x(), self.center.y());

        // Calculate the angle between the two vectors nalgebra Point2
        let angle = target_vector.coords.angle(&center_vector.coords);

        // Decide if we should walk around the center clockwise or counterclockwise
        if target_relative.y() > 0.0 {
            step = Step {
                forward: 0.0,
                left: 1.0,
                turn: 15.0,
            };
        } else {
            step = Step {
                forward: 0.0,
                left: -1.0,
                turn: -15.0,
            };
        };

        // Calculate the

        walking_engine.request_walk(step);
    }
}
