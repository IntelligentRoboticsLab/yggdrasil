use std::io::{self, BufRead, BufReader};
use std::time::Duration;
use tokio;
use tokio::io::AsyncBufReadExt;

use crate::filter::imu::IMUValues;
use crate::motion::motion_executer::reached_position;
use crate::motion::motion_manager::MotionManager;
use crate::motion::motion_types::{Motion, MotionType, Movement};

use miette::{IntoDiagnostic, Result};
use nidhogg::{
    types::{FillExt, JointArray, LeftEar, RightEar},
    NaoControlMessage, NaoState,
};
use serde_json;
use tyr::prelude::*;

pub struct DamagePreventionModule;

impl Module for DamagePreventionModule {
    fn initialize(self, app: App) -> Result<App> {
        app.add_system(fallconfirm)
            // .add_system(fallingstate)
            // .add_system(keyframe_recorder)
            .add_system(joint_locking_recorder)
            .add_task::<AsyncTask<String>>()?
            .add_resource(Resource::new(DamPrevResources {
                acc_values: [0f32; 50],
                acc_iterator: 0,
                brace_for_impact: true,
                active_motion: None,
            }))?
            .add_resource(Resource::new(RecordingResources {
                locked: false,
                total_keyframes: 0,
                motion: Motion {
                    initial_position: JointArray::<f32>::fill(0.0),
                    movements: Vec::new(),
                },
            }))
    }
}

const MAXIMUM_DEVIATION: f32 = 0.175;
const POSITION_ERROR_MARGIN: f32 = 0.20;

pub struct DamPrevResources {
    pub acc_values: [f32; 50],
    pub acc_iterator: u32,
    pub brace_for_impact: bool,
    pub active_motion: Option<Motion>,
}

pub struct RecordingResources {
    pub motion: Motion,
    pub locked: bool,
    pub total_keyframes: u32,
}

// fn standup(
//     damprevresources: &mut DamPrevResources,
//     imu_values: &IMUValues,
//     mmng: &mut MotionManager,
// ) {
//     if !damprevresources.brace_for_impact {
//         let motion_type = match lying_down(&imu_values, &damprevresources.acc_values) {
//             0 => Some(MotionType::Neutral),
//             1 => Some(MotionType::Neutral),
//             _ => None,
//         };

//         match motion_type {
//             Some(selected_motion) => {
//                 match mmng.get_active_motion() {}
//                 mmng.start_new_motion(selected_motion);
//             }
//             _ => {}
//         }
//     }
// }

#[system]
fn fallingstate(
    imu_values: &IMUValues,
    mmng: &mut MotionManager,
    damprevresources: &mut DamPrevResources,
    nao_state: &NaoState,
    nao_control_message: &mut NaoControlMessage,
) -> Result<()> {
    if damprevresources.brace_for_impact {
        let motion_type = match (
            imu_values.angles.y > 0.5,
            imu_values.angles.x > 0.5,
            imu_values.angles.y < -0.5,
            imu_values.angles.x < -0.5,
        ) {
            (true, false, _, false) => Some(MotionType::FallForwards), // forwards middle
            (true, false, _, true) => Some(MotionType::FallForwards),  // forwards left
            (true, true, _, false) => Some(MotionType::FallForwards),  // forwards right
            (_, false, true, false) => Some(MotionType::FallBackwards), // backwards middle
            (_, false, true, true) => Some(MotionType::FallBackwards), // backwards left
            (_, true, true, false) => Some(MotionType::FallBackwards), // backwards right
            (false, _, false, true) => Some(MotionType::FallLeftways), // left
            (false, true, false, _) => Some(MotionType::FallRightways), // right
            (_, _, _, _) => None,
        };

        // setting the active_motion variable to the final keyframe that the current motion is working towards
        match motion_type {
            Some(selected_motion) => {
                mmng.start_new_motion(selected_motion);
                match mmng.get_active_motion() {
                    Some(active_motion) => {
                        damprevresources.active_motion = Some(active_motion.motion)
                    }
                    _ => (),
                }

                damprevresources.brace_for_impact = false;
            }
            None => (),
        }
    }

    // comparing the current position to the final position of the falling motion, so we know when the falling position has been reached
    // when this is the case, the stiffness is set to zero.
    match &damprevresources.active_motion {
        Some(motion) => match motion.movements.last() {
            Some(final_movement) => {
                if reached_position(
                    &nao_state.position,
                    &final_movement.target_position,
                    POSITION_ERROR_MARGIN,
                ) {
                    nao_control_message.stiffness = JointArray::<f32>::fill(0.0)
                }
            }
            _ => {}
        },
        None => (),
    }

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

async fn read_command() -> String {
    let mut input = String::new();
    let stdin = tokio::io::stdin();
    // Create a buffered wrapper, which implements BufRead
    let mut reader = tokio::io::BufReader::new(stdin);
    // Take a stream of lines from this
    let _ = reader.read_line(&mut input);

    print!("{:?}", input);

    input
}

fn dispatch_command(task: &mut AsyncTask<String>) -> Result<()> {
    match task.try_spawn(read_command()) {
        // Dispatched
        Ok(_) => Ok(()),

        Err(Error::AlreadyActive) => Ok(()),
    }
}

#[system]
fn joint_locking_recorder(
    nao_state: &NaoState,
    nao_control_message: &mut NaoControlMessage,
    recordingresources: &mut RecordingResources,
    task: &mut AsyncTask<String>,
) -> Result<()> {
    dispatch_command(task)?;
    let Some(command) = task.poll() else {
        return Ok(());
    };

    let args: Vec<&str> = command.split(' ').collect();

    match args[0] {
        "lock" => match args[1] {
            "all" => {
                nao_control_message.stiffness = JointArray::<f32>::fill(0.3);
                print!("STUCK!");
            }
            _ => {}
        },
        "unlock" => match args[1] {
            "all" => {
                nao_control_message.stiffness = JointArray::<f32>::fill(0.0);
                print!("Free movement!");
            }
            _ => {}
        },
        "keyframe" => {
            let new_movement: Movement = Movement {
                target_position: nao_state.position.clone(),
                duration: Duration::new(1, 0),
            };
            recordingresources.motion.movements.push(new_movement);
            recordingresources.total_keyframes += 1;
            print!(
                "Frame added; Total: {:?}",
                recordingresources.total_keyframes
            )
        }
        "new" => {
            recordingresources.motion.initial_position = nao_state.position.clone();
            recordingresources.motion.movements.clear();
            recordingresources.total_keyframes = 0;
            print!("Motion Initialised!");
        }
        "print" => {
            let motion_json = serde_json::to_string(&recordingresources.motion);
            println!("{:?}", motion_json);
        }
        _ => {}
    }
    Ok(())
}
