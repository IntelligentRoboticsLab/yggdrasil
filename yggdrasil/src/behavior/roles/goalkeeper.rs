use bevy::prelude::*;
use heimdall::{Bottom, Top};
use nalgebra::{Point2, Point3, UnitComplex, Vector2};

use crate::{
    behavior::{
        behaviors::{Observe, WalkTo},
        engine::{in_role, CommandsBehaviorExt, RoleState, Roles},
    },
    core::config::layout::{FieldConfig, LayoutConfig},
    localization::RobotPose,
    motion::{
        step_planner::{StepPlanner, Target},
        walking_engine::{
            step::Step,
            step_context::{self, StepContext},
        },
    },
    vision::ball_detection::classifier::Balls,
};

const GOAL_POST_DISTANCE_OFFSET: f32 = 0.1;

/// Plugin for the Goalkeeper role
pub struct GoalkeeperRolePlugin;

impl Plugin for GoalkeeperRolePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, goalkeeper_role.run_if(in_role::<Goalkeeper>));
    }
}

/// The [`Goalkeeper`] role is held by a single robot at a time, usually player number 1.
/// It's job is to prevent the ball from entering the goal, which it does by staying in the goal area.
#[derive(Resource)]
pub struct Goalkeeper;
impl Roles for Goalkeeper {
    const STATE: RoleState = RoleState::Goalkeeper;
}

fn default_keeper_position(
    commands: &mut Commands,
    field_config: &FieldConfig,
    step_planner: &StepPlanner,
) {
    let keeper_target = Target {
        position: Point2::new(-field_config.length / 2.0, 0.0),
        rotation: Some(UnitComplex::<f32>::from_angle(0.0)),
    };

    if step_planner
        .current_absolute_target()
        .is_some_and(|current_target| *current_target == keeper_target)
        && step_planner.reached_target()
    {
        commands.set_behavior(Observe::default());
    } else {
        commands.set_behavior(WalkTo::new(keeper_target));
    }
}

fn block_ball(
    commands: &mut Commands,
    ball_position: Point2<f32>,
    field_config: &FieldConfig,
    step_context: &mut StepContext,
    robot_pose: &RobotPose,
) {
    let max_y_position = field_config.goal_width / 2.0
        - field_config.goal_post_diameter / 2.0
        - GOAL_POST_DISTANCE_OFFSET;
    let y_target = ball_position.y.clamp(-max_y_position, max_y_position);

    //let keeper_position = Point2::new(-field_config.length / 2.0, y_target);
    //let keeper_rotation = UnitComplex::<f32>::rotation_between(
    //    &Vector2::default(),
    //    &Vector2::new(
    //        ball_position.x - keeper_position.x,
    //        ball_position.y - keeper_position.y,
    //    ),
    //);
    //let keeper_target = Target {
    //    position: keeper_position,
    //    rotation: Some(keeper_rotation),
    //};
    //
    //commands.set_behavior(WalkTo::new_with_look_at(
    //    keeper_target,
    //    Point3::new(ball_position.x, ball_position.y, 0.0),
    //));

    let current_y = robot_pose.world_position().y;
    let step = if current_y > y_target {
        Step {
            forward: 0.0,
            left: 0.1,
            turn: 0.0,
        }
    } else {
        Step {
            forward: 0.0,
            left: -0.1,
            turn: 0.0,
        }
    };

    step_context.request_walk(step);
}

pub fn goalkeeper_role(
    mut commands: Commands,
    layout_config: Res<LayoutConfig>,
    step_planner: ResMut<StepPlanner>,
    top_balls: Res<Balls<Top>>,
    bottom_balls: Res<Balls<Bottom>>,
    mut step_context: ResMut<StepContext>,
    robot_pose: Res<RobotPose>,
) {
    let most_confident_ball = bottom_balls
        .most_confident_ball()
        .map(|b| b.position)
        .or(top_balls.most_confident_ball().map(|b| b.position));

    if let Some(ball_position) = most_confident_ball {
        block_ball(
            &mut commands,
            ball_position,
            &layout_config.field,
            &mut step_context,
            &robot_pose,
        );
    } else {
        default_keeper_position(&mut commands, &layout_config.field, &step_planner);
    }
}
