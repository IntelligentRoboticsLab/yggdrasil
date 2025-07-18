use bevy::prelude::*;
use nalgebra::Vector2;

use crate::{
    behavior::engine::{Behavior, BehaviorState, in_behavior},
    motion::keyframe::{KeyframeExecutor, MotionType},
    nao::{NaoManager, Priority},
    prelude::PreWrite,
    sensor::{
        falling::{FallState, LyingDirection},
        imu::IMUValues,
        low_pass_filter::ExponentialLpf,
    },
};

/// Behavior dedicated to handling the getup sequence of the robot.
/// The behavior will be entered once the robot is confirmed to be lying down,
/// this will execute the appropriate standup motion after which the robot will return
/// to the appropriate next behavior.
#[derive(Resource, Default)]
pub struct Standup {
    completed: bool,
}

impl Standup {
    #[must_use]
    pub fn completed(&self) -> bool {
        self.completed
    }
}

impl Behavior for Standup {
    const STATE: BehaviorState = BehaviorState::Standup;
}
pub struct StandupBehaviorPlugin;

impl Plugin for StandupBehaviorPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<StandupBalanceAdjustment>();
        app.add_systems(
            Update,
            (update_filtered_gyroscope, standup)
                .chain()
                .run_if(in_behavior::<Standup>),
        );

        app.add_systems(PreWrite, balance_standup.run_if(in_behavior::<Standup>));
    }
}

fn standup(
    mut standup: ResMut<Standup>,
    fall_state: Res<FallState>,
    mut keyframe_executor: ResMut<KeyframeExecutor>,
) {
    // check the direction the robot is lying and execute the appropriate motion
    match fall_state.as_ref() {
        FallState::Lying(LyingDirection::FacingDown) => {
            keyframe_executor.start_new_motion(MotionType::StandupStomach, Priority::High);
        }
        FallState::Lying(LyingDirection::FacingUp) => {
            keyframe_executor.start_new_motion(MotionType::StandupBack, Priority::High);
        }
        // if we are not lying down anymore, either standing up or falling, we do not execute any motion
        _ => {}
    }

    // Update completed status based on motion activity
    standup.completed = !keyframe_executor.is_motion_active();
}

#[derive(Resource)]
pub struct StandupBalanceAdjustment {
    pub filtered_gyro: ExponentialLpf<3>,
}

impl Default for StandupBalanceAdjustment {
    fn default() -> Self {
        Self {
            filtered_gyro: ExponentialLpf::new(0.3),
        }
    }
}

fn update_filtered_gyroscope(
    mut balance_adjustment: ResMut<StandupBalanceAdjustment>,
    imu: Res<IMUValues>,
) {
    balance_adjustment.filtered_gyro.update(imu.gyroscope);
}

fn balance_standup(
    standup: Res<Standup>,
    mut nao_manager: ResMut<NaoManager>,
    balance_adjustment: ResMut<StandupBalanceAdjustment>,
) {
    if standup.completed {
        return;
    }

    let settings = nao_manager.current_legs();
    let mut joints_position = settings.joints_position.clone();
    let joints_stiffness = settings.joints_stiffness.clone();

    let gyro = balance_adjustment.filtered_gyro.state();

    let balancing_factor = Vector2::new(0.0, 0.05);

    joints_position.left_leg.ankle_pitch += balancing_factor.y * gyro.y;
    joints_position.left_leg.ankle_roll += balancing_factor.x * gyro.x;
    joints_position.left_leg.hip_yaw_pitch += balancing_factor.x * gyro.x;
    joints_position.right_leg.ankle_pitch += balancing_factor.y * gyro.y;
    joints_position.right_leg.ankle_roll += balancing_factor.x * gyro.x;

    nao_manager.set_legs(joints_position, joints_stiffness, Priority::Custom(99));
}
