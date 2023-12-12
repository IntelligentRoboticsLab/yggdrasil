use crate::filter::imu::IMUValues;
use crate::motion::motion_executer::reached_position;
use crate::motion::motion_manager::MotionManager;
use crate::motion::motion_types::{Motion, MotionType, Movement};
use miette::Result;
use tyr::prelude::*;

use nidhogg::{
    types::{FillExt, LeftEar, RightEar},
    NaoControlMessage, NaoState,
};

pub struct FallingFilter;

impl Module for FallingFilter {
    fn initialize(self, app: App) -> Result<App> {
        Ok(app
            .add_resource(Resource::new(FallingStateWrapper::default()))?
            .add_system(falling_filter))
    }
}

#[derive(Default)]
struct FallingStateWrapper {
    state: FallingState,
}

#[derive(Default)]
enum FallingState {
    Falling(Direction),
    #[default]
    Standing,
    Fallen(FallenPosition),
}

enum Direction {
    Forwards,
    Backwards,
    Leftwards,
    Rightwards,
}

enum FallenPosition {
    Up,
    Down,
}

const MAXIMUM_DEVIATION: f32 = 0.175;
const POSITION_ERROR_MARGIN: f32 = 0.20;

pub struct DamPrevResources {
    pub acc_values: [f32; 50],
    pub acc_iterator: u32,
    pub brace_for_impact: bool,
    pub active_motion: Option<Motion>,
}

#[system]
fn falling_filter(imu_values: &IMUValues, fallingstate: &mut FallingStateWrapper) -> Result<()> {
    fallingstate.state = match (
        imu_values.angles.y > 0.5,
        imu_values.angles.x > 0.5,
        imu_values.angles.y < -0.5,
        imu_values.angles.x < -0.5,
    ) {
        (true, false, _, false) => FallingState::Falling(Direction::Forwards), // forwards middle
        (true, false, _, true) => FallingState::Falling(Direction::Forwards),  // forwards left
        (true, true, _, false) => FallingState::Falling(Direction::Forwards),  // forwards right
        (_, false, true, false) => FallingState::Falling(Direction::Backwards), // backwards middle
        (_, false, true, true) => FallingState::Falling(Direction::Backwards), // backwards left
        (_, true, true, false) => FallingState::Falling(Direction::Backwards), // backwards right
        (false, _, false, true) => FallingState::Falling(Direction::Leftwards), // left
        (false, true, false, _) => FallingState::Falling(Direction::Rightwards), // right
        (_, _, _, _) => FallingState::Standing,
    };
    Ok(())
}

fn standard_deviation(array: &[f32]) -> f32 {
    let avg: f32 = array.iter().sum::<f32>() / array.len() as f32;

    let variance: f32 = array
        .iter()
        .map(|val| (val - avg) * (val - avg))
        .sum::<f32>()
        / array.len() as f32;

    variance
}

fn lying_down(imu_values: &IMUValues, acc_values: &[f32; 50]) -> i8 {
    if acc_values[49] != 0.0 {
        let variance = standard_deviation(acc_values);

        // lying on stomach
        if variance < MAXIMUM_DEVIATION && imu_values.angles.y >= 1.5 {
            return 0;
        // lying on back
        } else if variance < MAXIMUM_DEVIATION && imu_values.angles.y <= -1.5 {
            return 1;
        }
    }
    return 2;
}

#[system]
fn fallconfirm(
    imu_values: &IMUValues,
    control: &mut NaoControlMessage,
    damprevresources: &mut DamPrevResources,
) -> Result<()> {
    let acc_iterator = damprevresources.acc_iterator;

    damprevresources.acc_values[acc_iterator as usize] = imu_values.accelerometer.y;

    if lying_down(&imu_values, &damprevresources.acc_values) <= 1 {
        control.left_ear = LeftEar::fill(1.0);
        control.right_ear = RightEar::fill(1.0);
    } else {
        control.left_ear = LeftEar::fill(0.0);
        control.right_ear = RightEar::fill(0.0);
    }

    damprevresources.acc_iterator = (acc_iterator + 1) % 50;

    Ok(())
}
