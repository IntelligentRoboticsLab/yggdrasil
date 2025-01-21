use bevy::prelude::*;
use nidhogg::types::{color, FillExt, RightEye};

use crate::{
    behavior::engine::{in_behavior, Behavior, BehaviorState},
    impl_behavior,
    nao::{NaoManager, Priority},
};

#[derive(Resource, Clone, Debug, Default)]
pub struct Example {
    iter: i32,
}

impl Behavior for Example {
    const STATE: BehaviorState = BehaviorState::Example;
}

pub struct ExampleBehaviorPlugin;
impl Plugin for ExampleBehaviorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, example_behavior.run_if(in_behavior::<Example>));
    }
}

pub fn example_behavior(mut example: ResMut<Example>, mut nao_manager: ResMut<NaoManager>) {
    example.iter += 1;

    let right_eye = if example.iter < 100 {
        RightEye::fill(color::f32::RED)
    } else {
        RightEye::fill(color::f32::BLUE)
    };

    nao_manager.set_right_eye_led(right_eye, Priority::Medium);
}
