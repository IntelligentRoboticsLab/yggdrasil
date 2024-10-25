use nalgebra::{Point2, Point3};
use nidhogg::types::{FillExt, HeadJoints, RightEye};

use crate::{
    behavior::engine::{Behavior, Context, Control},
    core::debug::DebugContext,
    motion::step_planner::Target,
    nao::Priority,
    vision::color,
};

#[derive(Copy, Clone, PartialEq, Debug)]
pub struct WalkTo {
    pub target: Target,
}

fn log_target(target: &Target, dbg: &mut DebugContext) {
    if let Some(rotation) = target.rotation {
        let direction = rotation.transform_point(&Point2::new(0.1, 0.0));

        dbg.log(
            "odometry/target",
            &rerun::Arrows3D::from_vectors([(direction.x, direction.y, 0.0)])
                .with_origins([(target.position.x, target.position.y, 0.0)])
                .with_colors([rerun::Color::from_rgb(255, 0, 0)]),
        );
    } else {
        dbg.log(
            "odometry/target",
            &rerun::Points3D::new([(target.position.x, target.position.y, 0.0)])
                .with_colors([rerun::Color::from_rgb(255, 0, 0)]),
        );
    }
}

impl Behavior for WalkTo {
    fn execute(&mut self, context: Context, control: &mut Control) {
        let target_point = Point3::new(self.target.position.x, self.target.position.y, 0.0);

        control.nao_manager.set_right_eye_led(
            RightEye::fill(nidhogg::types::color::f32::RED),
            Priority::default(),
        );

        let look_at = context.pose.get_look_at_absolute(&target_point);
        control
            .nao_manager
            .set_head(look_at, HeadJoints::fill(0.5), Priority::High);
        log_target(&self.target, &mut control.debug_context);

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
