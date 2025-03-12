//! See [`BallClassifierPlugin`].

use std::marker::PhantomData;
use std::time::{Duration, Instant};

use bevy::prelude::*;
use filter::{StateTransform, StateVector, UnscentedKalmanFilter};
use heimdall::{Bottom, CameraLocation, CameraMatrix, Top};
use itertools::Itertools;
use ml::prelude::ModelExecutor;
use nalgebra::{point, vector, Point2, Vector2, VectorSlice2};

use rerun::external::glam::Vec2;
use rerun::external::uuid::Timestamp;
use rerun::time;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use serde_with::DurationMilliSeconds;

use crate::core::debug::DebugContext;
use crate::localization::RobotPose;

use crate::nao::Cycle;
use crate::vision::camera::init_camera;
use ml::prelude::*;

use super::proposal::BallProposals;
use super::BallDetectionConfig;

const IMAGE_INPUT_SIZE: usize = 32;

#[serde_as]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BallClassifierConfig {
    pub confidence_threshold: f32,
    pub time_budget: usize,
    #[serde_as(as = "DurationMilliSeconds<u64>")]
    pub ball_life: Duration,
}

/// Plugin for classifying ball proposals produced by [`super::proposal::BallProposalPlugin`].
///
/// This plugin uses a cnn model to classify whether the proposals are balls or not.
pub struct BallClassifierPlugin;

impl Plugin for BallClassifierPlugin {
    fn build(&self, app: &mut App) {
        app.init_ml_model::<BallClassifierModel>()
            .add_systems(
                PostStartup,
                (
                    init_ball_classifier::<Top>.after(init_camera::<Top>),
                    init_ball_classifier::<Bottom>.after(init_camera::<Bottom>),
                ),
            )
            .add_systems(
                Update,
                (
                    classify_balls::<Top>.run_if(resource_exists_and_changed::<BallProposals<Top>>),
                    classify_balls::<Bottom>
                        .run_if(resource_exists_and_changed::<BallProposals<Bottom>>),
                ),
            )
            .add_systems(
                PostUpdate,
                (
                    log_ball_classifications::<Top>
                        .run_if(resource_exists_and_changed::<Balls<Top>>),
                    log_ball_classifications::<Bottom>
                        .run_if(resource_exists_and_changed::<Balls<Bottom>>),
                ),
            );
    }
}

fn init_ball_classifier<T: CameraLocation>(mut commands: Commands) {
    let balls: Balls<T> = Balls {
        balls: Vec::new(),
        cycle: Cycle::default(),
    };

    commands.insert_resource(balls);
}

fn log_ball_classifications<T: CameraLocation>(dbg: DebugContext, balls: Res<Balls<T>>) {
    let (positions, (half_sizes, confidences)): (Vec<_>, (Vec<_>, Vec<_>)) = balls
        .balls
        .iter()
        .filter(|ball| ball.cycle == balls.cycle)
        .map(|ball| {
            (
                Into::<Vec2>::into(ball.position_image),
                (
                    (ball.scale / 2.0, ball.scale / 2.0),
                    format!("{:.2}", ball.confidence),
                ),
            )
        })
        .unzip();

    if positions.is_empty() {
        dbg.log_with_cycle(
            T::make_entity_image_path("balls/classifications"),
            balls.cycle,
            &rerun::Clear::flat(),
        );
        return;
    }
    dbg.log_with_cycle(
        T::make_entity_image_path("balls/classifications"),
        balls.cycle,
        &rerun::Boxes2D::from_centers_and_half_sizes(positions, &half_sizes)
            .with_labels(confidences),
    );
}

pub(super) struct BallClassifierModel;

impl MlModel for BallClassifierModel {
    type Inputs = Vec<f32>;
    type Outputs = f32;

    const ONNX_PATH: &'static str = "models/ball_classifier.onnx";
}

#[derive(Debug)]
pub struct Ball<T: CameraLocation> {
    /// The position of the ball proposal in the image, in pixels.
    pub position_image: Point2<f32>,
    /// The vector from the robot to the ball proposal, in robot frame.
    pub robot_to_ball: Vector2<f32>,
    /// The absolute position of the ball proposal, in world frame.
    pub position: Point2<f32>,
    /// Velocity of the ball proposal, in world frame.
    pub velocity: Vector2<f32>,
    /// The scale of the ball proposal.
    pub scale: f32,
    /// The distance to the ball in meters, at the time of detection.
    pub distance: f32,
    /// The timestamp the ball was detected at.
    pub timestamp: Instant,
    /// The confidence score assigned to the detected ball.
    pub confidence: f32,
    /// The cycle of the image this ball was detected in.
    pub cycle: Cycle,
    _marker: PhantomData<T>,
}

// NOTE: This needs to be implemented manually because of the `PhantomData`
// https://github.com/rust-lang/rust/issues/26925
impl<T: CameraLocation> Clone for Ball<T> {
    fn clone(&self) -> Self {
        Self {
            position_image: self.position_image,
            robot_to_ball: self.robot_to_ball,
            position: self.position,
            velocity: self.velocity,
            scale: self.scale,
            distance: self.distance,
            timestamp: self.timestamp,
            confidence: self.confidence,
            cycle: self.cycle,
            _marker: PhantomData,
        }
    }
}

#[derive(Resource, Clone)]
pub struct Balls<T: CameraLocation> {
    pub balls: Vec<Ball<T>>,
    pub cycle: Cycle,
}

impl<T: CameraLocation> Balls<T> {
    #[must_use]
    pub fn no_balls(&self) -> bool {
        self.balls.is_empty()
    }

    #[must_use]
    pub fn most_confident_ball(&self) -> Option<&Ball<T>> {
        self.balls
            .iter()
            .reduce(|a, b| if a.confidence > b.confidence { a } else { b })
    }

    #[must_use]
    pub fn most_recent_ball(&self) -> Option<&Ball<T>> {
        self.balls
            .iter()
            .reduce(|a, b| if a.timestamp > b.timestamp { a } else { b })
    }
}

#[derive(Deref, DerefMut, Clone, Copy)]
struct BallPosition(Point2<f32>);

impl From<BallPosition> for StateVector<2> {
    fn from(value: BallPosition) -> Self {
        value.xy().coords
    }
}

impl From<StateVector<2>> for BallPosition {
    fn from(value: StateVector<2>) -> Self {
        BallPosition(point![value.x, value.y])
    }
}

impl StateTransform<2> for BallPosition {} 

fn classify_balls<T: CameraLocation>(
    mut commands: Commands,
    mut proposals: ResMut<BallProposals<T>>,
    mut model: ResMut<ModelExecutor<BallClassifierModel>>,
    mut balls: ResMut<Balls<T>>,
    camera_matrix: Res<CameraMatrix<T>>,
    config: Res<BallDetectionConfig>,
    robot_pose: Res<RobotPose>,
) {
    let classifier = &config.classifier;
    let start = Instant::now();

    let mut classified_balls = Vec::new();

    for proposal in proposals
        .proposals
        .drain(..)
        .sorted_by(|a, b| a.distance_to_ball.total_cmp(&b.distance_to_ball))
    {
        if proposal.distance_to_ball > 20.0 {
            continue;
        }

        if start.elapsed().as_micros() > classifier.time_budget as u128 {
            break;
        }

        let patch_size = proposal.scale as usize;
        let patch = proposals.image.get_grayscale_patch(
            (proposal.position.x, proposal.position.y),
            patch_size,
            patch_size,
        );

        let patch = ml::util::resize_patch(
            (patch_size, patch_size),
            (IMAGE_INPUT_SIZE, IMAGE_INPUT_SIZE),
            patch,
        );

        // sigmoid is applied in model onnx
        let confidence = commands
            .infer_model(&mut model)
            .with_input(&patch)
            .spawn_blocking(|output| 1.0 - output);

        if confidence < classifier.confidence_threshold {
            continue;
        }

        let Ok(robot_to_ball) = camera_matrix.pixel_to_ground(proposal.position.cast(), 0.0) else {
            tracing::warn!(?proposal.position, "failed to project ball position to ground");
            continue;
        };

        let position = BallPosition(robot_pose.robot_to_world(&Point2::from(robot_to_ball.xy())));

        let cov = nalgebra::SMatrix::<f32, 2, 2>::from_diagonal_element(0.05);
        let mut ukf = UnscentedKalmanFilter::<2, 5, BallPosition>::new(position, cov);

        

        let timestamp = Instant::now();

        let velocity = if let Some(most_recent_ball) = balls.most_confident_ball() {
            let time_diff = timestamp - most_recent_ball.timestamp;
            let distance_diff = position.0 - most_recent_ball.position;
            distance_diff / time_diff.as_secs_f32()
        } else {
            Vector2::zeros()
        };

        classified_balls.push(Ball {
            position_image: proposal.position.cast(),
            robot_to_ball: robot_to_ball.xy().coords,
            scale: proposal.scale,
            position: robot_pose.robot_to_world(&Point2::from(robot_to_ball.xy())),
            velocity: velocity,
            distance: proposal.distance_to_ball,
            timestamp: Instant::now(),
            cycle: proposals.image.cycle(),
            confidence,
            _marker: PhantomData,
        });

        // TODO: we only store the closest ball with high enough confidence
        // Maybe we should store multiple candidates.
        break;
    }

    if classified_balls.is_empty() {
        for ball in &balls.balls {
            if ball.timestamp.elapsed() < classifier.ball_life {
                // keep the ball if it isn't too old, but mark it as not fresh
                classified_balls.push(ball.clone());
            }
        }
    }

    balls.balls = classified_balls;
    balls.cycle = proposals.image.cycle();
}
