use nidhogg::types::{FillExt, HeadJoints};

use crate::{
    behavior::engine::{Behavior, Context, Control},
    motion::step_planner::Target,
    nao::manager::Priority,
};

#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct WalkToSet;

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

        let target = Target {
            position: set_robot_position.isometry.translation.vector.into(),
            rotation: Some(set_robot_position.isometry.rotation),
        };

        control.step_planner.set_absolute_target_if_unset(target);
        if let Some(step) = control.step_planner.plan(context.pose) {
            control.walking_engine.request_walk(step);
        } else {
            control.walking_engine.request_stand();
        }
    }
}
