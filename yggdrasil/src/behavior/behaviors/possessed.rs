use bevy::prelude::*;

use crate::{
    behavior::engine::{in_behavior, Behavior, BehaviorState},
    motion::walking_engine::{step::Step, step_context::StepContext, Gait},
};

pub struct PossessedBehaviorPlugin;

impl Plugin for PossessedBehaviorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, possessed.run_if(in_behavior::<Possessed>));
    }
}

const POSSESSED_WALK_SPEED: f32 = 0.045;
const POSSESSED_SIDE_SPEED: f32 = 0.045;
const POSSESSED_TURN_SPEED: f32 = 0.2;
const POSSESSED_LEFT_DEADBAND: f32 = 0.1;
const POSSESSED_RIGHT_DEADBAND: f32 = 0.1;

#[derive(Resource)]
pub struct Possessed;

impl Behavior for Possessed {
    const STATE: BehaviorState = BehaviorState::Possessed;
}

pub fn possessed(
    gamepad: Query<&Gamepad>,
    gait: Res<State<Gait>>,
    mut step_context: ResMut<StepContext>,
) {
    let gamepad = gamepad.single();

    if gamepad.just_pressed(GamepadButton::West) {
        match **gait {
            Gait::Sitting => step_context.request_stand(),
            _ => step_context.request_sit(),
        }
    }

    let left_stick = if gamepad.left_stick().length() > POSSESSED_LEFT_DEADBAND {
        gamepad.left_stick()
    } else {
        Vec2::ZERO
    };

    let right_stick = if gamepad.right_stick().length() > POSSESSED_RIGHT_DEADBAND {
        Vec2::Y.angle_to(gamepad.right_stick())
    } else {
        0.
    };

    if !matches!(**gait, Gait::Sitting) {
        step_context.request_walk(Step {
            forward: POSSESSED_WALK_SPEED * left_stick.y,
            left: -POSSESSED_SIDE_SPEED * left_stick.x,
            turn: POSSESSED_TURN_SPEED * right_stick,
        });
    }
}
