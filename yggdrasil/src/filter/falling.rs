use crate::filter::imu::IMUValues;
use miette::Result;
use tyr::prelude::*;

use nidhogg::{
    types::{FillExt, LeftEar, RightEar},
    NaoControlMessage,
};

pub struct FallingFilter;

impl Module for FallingFilter {
    fn initialize(self, app: App) -> Result<App> {
        app.add_system(pose_filter)
            .add_resource(Resource::new(Pose::default()))
    }
}

#[derive(Default)]
pub struct Pose {
    pub state: PoseState,
}

#[derive(Default, Clone, Debug)]
pub enum PoseState {
    Falling(FallDirection),
    #[default]
    Upright,
    Lying(LyingFacing),
}
#[derive(Clone, Debug)]
pub enum FallDirection {
    Forwards,
    Backwards,
    Leftways,
    Rightways,
}

#[derive(Clone, Debug)]
pub enum LyingFacing {
    Up,
    Down,
}

const MAXIMUM_DEVIATION: f32 = 0.175;

#[system]
fn pose_filter(
    imu_values: &IMUValues,
    fallingstate: &mut Pose,
    control: &mut NaoControlMessage,
) -> Result<()> {
    fallingstate.state = match (
        imu_values.angles.y > 0.6,
        imu_values.angles.y < -0.6,
        imu_values.angles.x > 0.6,
        imu_values.angles.x < -0.6,
        imu_values.gyroscope.y.abs() > 1.0,
        imu_values.gyroscope.x.abs() > 1.0,
    ) {
        (true, _, _, _, true, _) => PoseState::Falling(FallDirection::Forwards), // forwards
        (_, true, _, _, true, _) => PoseState::Falling(FallDirection::Backwards), // backwards
        (_, _, true, _, _, true) => PoseState::Falling(FallDirection::Rightways), // right
        (_, _, _, true, _, true) => PoseState::Falling(FallDirection::Leftways), // left
        (_, _, _, _, _, _) => fallingstate.state.clone(),
    };

    // lying on stomach
    if imu_values.accelerometer_std.y < MAXIMUM_DEVIATION && imu_values.angles.y >= 1.5 {
        fallingstate.state = PoseState::Lying(LyingFacing::Down);
        control.left_ear = LeftEar::fill(1.0);
        control.right_ear = RightEar::fill(1.0);

    // lying on back
    } else if imu_values.accelerometer_std.y < MAXIMUM_DEVIATION && imu_values.angles.y <= -1.5 {
        fallingstate.state = PoseState::Lying(LyingFacing::Up);
        control.left_ear = LeftEar::fill(1.0);
        control.right_ear = RightEar::fill(1.0);
    } else {
        control.left_ear = LeftEar::fill(0.0);
        control.right_ear = RightEar::fill(0.0);
    }

    Ok(())
}
