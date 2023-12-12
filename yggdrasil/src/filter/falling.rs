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
        Ok(app
            .add_system(pose_filter)
            .add_resource(Resource::new(Pose::default()))?)
    }
}

#[derive(Default)]
struct Pose {
    state: PoseState,
}

#[derive(Default)]
enum PoseState {
    Falling(FallDirection),
    #[default]
    Upright,
    Lying(LyingFacing),
}

enum FallDirection {
    Forwards,
    Backwards,
    Leftways,
    Rightways,
}

enum LyingFacing {
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
        imu_values.angles.y > 0.5,
        imu_values.angles.x > 0.5,
        imu_values.angles.y < -0.5,
        imu_values.angles.x < -0.5,
    ) {
        (true, false, _, false) => PoseState::Falling(FallDirection::Forwards), // forwards middle
        (true, false, _, true) => PoseState::Falling(FallDirection::Forwards),  // forwards left
        (true, true, _, false) => PoseState::Falling(FallDirection::Forwards),  // forwards right
        (_, false, true, false) => PoseState::Falling(FallDirection::Backwards), // backwards middle
        (_, false, true, true) => PoseState::Falling(FallDirection::Backwards), // backwards left
        (_, true, true, false) => PoseState::Falling(FallDirection::Backwards), // backwards right
        (false, _, false, true) => PoseState::Falling(FallDirection::Leftways), // left
        (false, true, false, _) => PoseState::Falling(FallDirection::Rightways), // right
        (_, _, _, _) => PoseState::Upright,
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
