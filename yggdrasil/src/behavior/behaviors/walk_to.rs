use std::time::Duration;

use nalgebra::Point3;

use crate::{
    behavior::engine::{Behavior, Context, Control},
    motion::step_planner::Target,
    nao::Priority,
};

const HEAD_ROTATION_TIME: Duration = Duration::from_millis(500);
const HEAD_STIFFNESS: f32 = 0.2;

/// Walk to a target position using the step planner, whilst looking at the target.
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct WalkTo {
    pub target: Target,
}

impl Behavior for WalkTo {
    fn execute(&mut self, context: Context, control: &mut Control) {
        let target_point = Point3::new(self.target.position.x, self.target.position.y, 0.0);

        let look_at = context.pose.get_look_at_absolute(&target_point);
        control.nao_manager.set_head_target(
            look_at,
            HEAD_ROTATION_TIME,
            Priority::default(),
            HEAD_STIFFNESS,
        );

        if control
            .step_planner
            .current_absolute_target()
            .is_some_and(|target| target != &self.target)
        {
            control.step_planner.clear_target();
        }

        control
            .step_planner
            .set_absolute_target_if_unset(self.target);
        if let Some(step) = control.step_planner.plan(context.pose) {
            control.walking_engine.request_walk(step);
        } else {
            control.walking_engine.request_stand();
        }
    }
}
