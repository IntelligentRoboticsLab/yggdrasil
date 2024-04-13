use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use std::time::Instant;

use crate::{
    behavior::engine::{Behavior, Context},
    nao::manager::{NaoManager, Priority},
    walk::engine::{Step, WalkingEngine},
};
use nidhogg::types::{color::f32::GREEN, FillExt, HeadJoints};

const ROTATION_STIFFNESS: f32 = 0.3;

/// Config struct containing parameters for the initial behavior.
#[serde_as]
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct ObserveBehaviorConfig {
    // Controls how fast the robot moves its head back and forth while looking around
    pub head_rotation_speed: f32,
    // Controls how far to the left and right the robot looks while looking around, in radians.
    // If this value is one, the robot will look one radian to the left and one radian to the
    // right.
    pub head_pitch_max: f32,
    // Controls how far to the bottom the robot looks while looking around, in radians
    pub head_yaw_max: f32,
}

#[derive(Copy, Clone, Debug)]
pub struct Observe {
    pub starting_time: Instant,
}

impl Default for Observe {
    fn default() -> Self {
        Observe {
            starting_time: Instant::now(),
        }
    }
}

impl Behavior for Observe {
    fn execute(
        &mut self,
        context: Context,
        nao_manager: &mut NaoManager,
        walking_engine: &mut WalkingEngine,
    ) {
        let ObserveBehaviorConfig {
            head_rotation_speed,
            head_pitch_max: head_pitch_multiplier,
            head_yaw_max: head_yaw_multiplier,
        } = context.behavior_config.observe;

        look_around(
            nao_manager,
            self.starting_time,
            head_rotation_speed,
            head_yaw_multiplier,
            head_pitch_multiplier,
        );

        nao_manager.set_chest_led(GREEN, Priority::High);

        walking_engine.request_walk(Step {
            forward: 0.04,
            left: 0.0,
            turn: 0.0,
        });
    }
}

fn look_around(
    nao_manager: &mut NaoManager,
    starting_time: Instant,
    rotation_speed: f32,
    yaw_multiplier: f32,
    pitch_multiplier: f32,
) {
    // Used to parameterize the yaw and pitch angles, multiplying with a large
    // rotation speed will make the rotation go faster.
    let movement_progress = starting_time.elapsed().as_secs_f32() * rotation_speed;
    let yaw = (movement_progress).sin() * yaw_multiplier;
    let pitch = (movement_progress * 2.0 + std::f32::consts::FRAC_PI_2)
        .sin()
        .max(0.0)
        * pitch_multiplier;

    let position = HeadJoints { yaw, pitch };
    let stiffness = HeadJoints::fill(ROTATION_STIFFNESS);

    nao_manager.set_head(position, stiffness, Priority::default());
}
