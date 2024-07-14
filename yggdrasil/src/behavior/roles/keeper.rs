use nalgebra::{Point2, UnitComplex};

use crate::{
    behavior::{
        behaviors::{Observe, Walk},
        engine::{BehaviorKind, Context, Control, Role},
    },
    core::config::layout::FieldConfig,
    motion::step_planner::Target,
};

pub struct Keeper;

fn is_close_to_own_goal(robot_position: &Point2<f32>, field: &FieldConfig) -> bool {
    let is_withing_goal_area_length = robot_position.x > field.length / 2.
        && robot_position.x < field.length / 2. + field.penalty_area_length;

    let is_within_goal_area_width = robot_position.y < field.penalty_area_width / 2.
        && robot_position.y > -field.penalty_area_width / 2.;

    is_withing_goal_area_length && is_within_goal_area_width
}

impl Role for Keeper {
    fn transition_behavior(&mut self, context: Context, _control: &mut Control) -> BehaviorKind {
        if is_close_to_own_goal(&context.pose.world_position(), &context.layout_config.field) {
            return BehaviorKind::Observe(Observe::default());
        }
        // target: Isometry2::new(vector!(-context.layout_config.field.length / 2., 0.), 0.0),

        BehaviorKind::Walk(Walk {
            target: Target {
                position: Point2::new(-context.layout_config.field.length / 2., 0.),
                rotation: Some(UnitComplex::<f32>::from_angle(0.0)),
            },
        })
    }
}
