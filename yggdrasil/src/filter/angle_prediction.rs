//! Joint angle prediction based on B-Human's model.

use crate::{
    ml::{MlModel, MlTask, MlTaskResource},
    nao::manager,
    prelude::*,
};

use nidhogg::{
    types::{JointArray, LeftLegJoints, LegJoints, RightLegJoints},
    NaoControlMessage, NaoState,
};
use std::collections::VecDeque;
use std::ops::Range;

pub struct AnglePredictionModule;

/// Number of requests and sensors timesteps to store in the ringbuffer.
const HISTORY_SIZE: usize = 4;
/// Range of the requests included in the window.
const REQUESTS_RANGE: Range<usize> = 0..3;
/// Range of the sensors included in the window.
const SENSORS_RANGE: Range<usize> = 1..4;

/// Size of the window which serves as the input to the model.
const WINDOW_SIZE: usize = REQUESTS_RANGE.end - REQUESTS_RANGE.start;

impl Module for AnglePredictionModule {
    fn initialize(self, app: App) -> Result<App> {
        // Some sanity checks (should be compile-time but oh well).
        assert_eq!(WINDOW_SIZE, SENSORS_RANGE.end - SENSORS_RANGE.start);
        assert!(REQUESTS_RANGE.end <= HISTORY_SIZE);
        assert!(SENSORS_RANGE.end <= HISTORY_SIZE);

        app.add_system(angle_prediction.after(manager::finalize))
            .init_debuggable_resource::<AnglePrediction>()?
            .add_ml_task::<AnglePredictionModel>()
    }
}

pub struct AnglePredictionModel;

impl MlModel for AnglePredictionModel {
    type InputType = f32;
    type OutputType = f32;

    /// Shamelessly stolen from
    /// `https://github.com/bhuman/BHumanCodeRelease/raw/50c83c6a8826ed2c08bc7400d9e9d76e48b343a9/Config/NeuralNets/JointAngle/lstm_i03l5u48.onnx`
    const ONNX_PATH: &'static str = "models/angle_prediction.onnx";
}

#[derive(Debug, Default)]
pub struct AnglePrediction {
    pub prediction: Option<LegJoints<f32>>,
    pub age: usize,
    requests: VecDeque<[f32; 11]>,
    sensors: VecDeque<[f32; 11]>,
}

#[system]
pub fn angle_prediction(
    model: &mut MlTask<AnglePredictionModel>,
    state: &NaoState,
    message: &NaoControlMessage,
    data: &mut AnglePrediction,
) -> Result<()> {
    // If the model is finished, publish the prediction.
    if let Some(result) = model.poll::<Vec<f32>>().transpose()? {
        data.prediction = Some(unpack_angles(&result));
        data.age = 0;
    } else {
        data.age += 1;
    }

    // Trim the ringbuffer so it's an actual ringbuffer.
    if data.requests.len() == HISTORY_SIZE {
        data.requests.pop_front();
    }
    if data.sensors.len() == HISTORY_SIZE {
        data.sensors.pop_front();
    }

    // Store the current requests and sensors data.
    data.requests.push_back(pack_angles(&message.position));
    data.sensors.push_back(pack_angles(&state.position));

    // Model and data are ready.
    if !model.active() && data.requests.len() == HISTORY_SIZE {
        let mut window = Vec::with_capacity(22 * WINDOW_SIZE);

        // Combine the requests and sensors into a single window.
        for (i, j) in REQUESTS_RANGE.zip(SENSORS_RANGE) {
            window.extend(&data.requests[i]);
            window.extend(&data.sensors[j]);
        }

        // Do it.
        model.try_start_infer(&window)?;
    }

    println!("{:?}", data);
    Ok(())
}

/// Helper function to pack the angles in the order the model expects.
fn pack_angles(position: &JointArray<f32>) -> [f32; 11] {
    [
        position.right_ankle_roll,
        position.right_ankle_pitch,
        position.right_knee_pitch,
        position.right_hip_pitch,
        position.right_hip_roll,
        position.left_knee_pitch,
        position.left_ankle_roll,
        position.left_ankle_pitch,
        position.left_hip_pitch,
        position.left_hip_roll,
        position.left_hip_yaw_pitch,
    ]
}

/// Helper function to unpack the angles from the order the model returns.
fn unpack_angles(raw: &[f32]) -> LegJoints<f32> {
    LegJoints {
        right_leg: RightLegJoints {
            ankle_roll: raw[0],
            ankle_pitch: raw[1],
            knee_pitch: raw[2],
            hip_pitch: raw[3],
            hip_roll: raw[4],
        },
        left_leg: LeftLegJoints {
            knee_pitch: raw[5],
            ankle_roll: raw[6],
            ankle_pitch: raw[7],
            hip_pitch: raw[8],
            hip_roll: raw[9],
            hip_yaw_pitch: raw[10],
        },
    }
}
