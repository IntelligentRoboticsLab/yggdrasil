use bevy::prelude::*;
use heimdall::Top;
use ml::{
    prelude::{MlTaskCommandsExt, ModelExecutor},
    MlModel, MlModelResourceExt,
};

use crate::vision::camera::Image;

use super::DetectRefereePose;

// TODO: Probably in a config file
const INPUT_WIDTH: u32 = 640;
const INPUT_HEIGHT: u32 = 640;

pub struct RefereePoseEstimatorPlugin;

impl Plugin for RefereePoseEstimatorPlugin {
    fn build(&self, app: &mut App) {
        app.init_ml_model::<RefereePoseEstimatorModel>()
            .add_event::<DetectRefereePose>()
            .add_event::<RefereePoseEstimated>()
            .add_systems(Update, (
                estimate_referee_pose,
                send_referee_pose_estimate_output,
        ));
    }
}

pub(super) struct RefereePoseEstimatorModel;

impl MlModel for RefereePoseEstimatorModel {
    type Inputs = Vec<f32>;

    type Outputs = (Vec<f32>, Vec<f32>, Vec<f32>, Vec<f32>, Vec<f32>);

    const ONNX_PATH: &'static str = "models/yolo11n-pose-simple.onnx";
}

pub fn estimate_referee_pose(
    mut commands: Commands,
    mut detect_pose: EventReader<DetectRefereePose>,
    mut model: ResMut<ModelExecutor<RefereePoseEstimatorModel>>,
    image: Res<Image<Top>>,
) {
    for _ev in detect_pose.read() {
        println!("Run estimator");
        // Convert the yuyv image to an rgb image
        let rgb_image = image
            .yuyv_image()
            .to_rgb()
            .expect("Failed to convert image from yuyv to rgb");

        // Resize the image to fit the model input
        let resized_image: Vec<u8> = rgb_image
            .resize(INPUT_WIDTH, INPUT_HEIGHT)
            .expect("Failed to resize image for robot detection");

        // let resized_image = image
        //     .yuyv_image()
        //     .resize(INPUT_WIDTH, INPUT_HEIGHT)
        //     .expect("Failed to resize image for robot detection");

        println!("Image is resized");

        let normalized_image: Vec<f32> = resized_image
            .to_vec()
            .iter()
            .map(|pixel| *pixel as f32 / 255.0)
            .collect();

        println!("Image is normalized");

        commands
            .infer_model(&mut model)
            .with_input(&normalized_image)
            .create_resource()
            .spawn(|_model_output| {
                let estimated_pose: Vec<f32> = vec![0.; 17 * 2];
                let output = RefereePoseEstimationOutput { estimated_pose };
                Some(output)
            });

        break;
    }

    // A single estimate is enough for multiple estimate pose requests so extra
    // events can all be removed.
    detect_pose.clear();
}

pub fn send_referee_pose_estimate_output(
    estimated_pose_output: Option<Res<RefereePoseEstimationOutput>>,
    mut pose_estimated: EventWriter<RefereePoseEstimated>,
) {
    if let Some(estimated_pose_output) = estimated_pose_output {
        if estimated_pose_output.is_added() || estimated_pose_output.is_changed() {
            println!("Ended pose estimation");
            pose_estimated.send(RefereePoseEstimated {
                estimated_pose: estimated_pose_output.estimated_pose.clone(),
            });
        }
    }
}

#[derive(Event)]
pub struct RefereePoseEstimated {
    pub estimated_pose: Vec<f32>,
}

#[derive(Resource)]
pub struct RefereePoseEstimationOutput {
    pub estimated_pose: Vec<f32>,
}
