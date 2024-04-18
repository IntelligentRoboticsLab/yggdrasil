use nalgebra::Point2;

use crate::{
    behavior::engine::{Behavior, Context},
    config::layout::WorldPosition,
    nao::manager::NaoManager,
    walk::engine::{Step, WalkingEngine},
};

#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct WalkTo {
    pub target: Point2<f32>,
}

impl Behavior for WalkTo {
    fn execute(
        &mut self,
        context: Context,
        _nao_manager: &mut NaoManager,
        walking_engine: &mut WalkingEngine,
    ) {
        // x is infront, y is to the left
        let target_relative = self.target.coords - context.robot_position.coords;

        let mut step = Step::default();

        // if target is right in front of us, walk straight
        if target_relative.y.abs() < 0.1 {
            println!("Walking straight");
            step = Step {
                forward: 1.0,
                left: 0.0,
                turn: 0.0,
            };
        } else {
            println!("Turn to target");
            // Calculate the angle to the target in radians
            let turn = target_relative.y.atan2(target_relative.x);

            // Calculate the distance to the target
            let forward = target_relative.x.hypot(target_relative.y);

            step = Step {
                forward: 0.0,
                left: 0.0,
                turn,
            };
        }

        walking_engine.request_walk(step);
    }
}
