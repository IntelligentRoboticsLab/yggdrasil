use nalgebra::Point2;
use nidhogg::types::{FillExt, HeadJoints};

use crate::{
    behavior::engine::{Behavior, Context, Control},
    motion::step_planner::Target,
    nao::manager::Priority,
};

const KEEPER_PRE_SET_POS: Target = Target {
    position: Point2::new(-2.85, 0.0),
    rotation: None,
};

#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct WalkToSet {
    pub is_keeper: bool,
}

impl Behavior for WalkToSet {
    fn execute(&mut self, context: Context, control: &mut Control) {
        let set_robot_position = context
            .layout_config
            .set_positions
            .player(context.player_config.player_number);

        let set_position = set_robot_position.isometry.translation.vector.into();

        let look_at = context.pose.get_look_at_absolute(&set_position);
        control
            .nao_manager
            .set_head(look_at, HeadJoints::fill(0.5), Priority::default());

        let target: Target = Target {
            position: set_robot_position.isometry.translation.vector.into(),
            rotation: Some(set_robot_position.isometry.rotation),
        };
        if self.is_keeper
            && (!control.step_planner.has_target()
                || (control
                    .step_planner
                    .current_absolute_target()
                    .is_some_and(|target| target == &KEEPER_PRE_SET_POS)
                    && !control.step_planner.reached_target()))
        {
            control.step_planner.set_absolute_target(KEEPER_PRE_SET_POS);
        } else {
            control.step_planner.set_absolute_target(target);
        }

        if let Some(step) = control.step_planner.plan(context.pose) {
            control.walking_engine.request_walk(step);
        } else {
            let look_at = context.pose.get_look_at_absolute(&Point2::origin());
            control
                .nao_manager
                .set_head(look_at, HeadJoints::fill(0.5), Priority::default());

            control.walking_engine.request_stand();
        }
    }
}
