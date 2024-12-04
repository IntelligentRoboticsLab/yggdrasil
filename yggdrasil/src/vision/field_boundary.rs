//! Module for detecting the field boundary lines from the top camera image
//!

use crate::{core::debug::DebugContext, vision::camera::Image};
use bevy::{app::Plugin, prelude::*};
use heimdall::{CameraLocation, Top};
use lstsq::Lstsq;
use ml::prelude::*;
use nalgebra::Point2;
use tasks::conditions::task_finished;

use super::camera::init_camera;

const MODEL_INPUT_WIDTH: u32 = 40;
const MODEL_INPUT_HEIGHT: u32 = 30;

/// Module for detecting the field boundary lines from the top camera image
///
/// It adds the following resources to the app:
/// - [`FieldBoundary`]
pub struct FieldBoundaryPlugin;

impl Plugin for FieldBoundaryPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.init_ml_model::<FieldBoundaryModel>()
            .add_systems(
                PostStartup,
                (init_field_boundary, setup_boundary_debug_logging).after(init_camera::<Top>),
            )
            .add_systems(
                Update,
                detect_field_boundary
                    .run_if(resource_exists_and_changed::<Image<Top>>)
                    .run_if(task_finished::<FieldBoundary>),
            )
            .add_systems(
                PostUpdate,
                log_boundary_points.run_if(resource_exists_and_changed::<FieldBoundary>),
            );
    }
}

/// Predicted points from the field boundary model
#[derive(Debug, Clone)]
struct FieldBoundaryPoint {
    x: f32,
    y: f32,
    /// Square root of the error of the prediction
    /// See <https://b-human.de/downloads/publications/2022/DeepFieldBoundary.pdf>
    error: f32,
}

/// Fitted field boundary lines
#[derive(Clone)]
pub enum FieldBoundaryLines {
    /// A field boundary made up of one line
    One { line: Line },
    /// A field boundary made up of two lines that intersect at their apex
    Two {
        left_line: Line,
        right_line: Line,
        intersection: Point2<f32>,
    },
}

/// A fitted field boundary from a given image
#[derive(Resource, Clone)]
pub struct FieldBoundary {
    /// The fitted field boundary lines
    pub lines: FieldBoundaryLines,
    /// Sum of squared residual error from fitting the boundary
    error: f32,
    /// The predicted points used to fit the boundary
    points: Vec<FieldBoundaryPoint>,
    /// The image the boundary was predicted from
    pub image: Image<Top>,
}

impl FieldBoundary {
    /// Get the height of the boundary at a given horizontal pixel
    #[must_use]
    pub fn height_at_pixel(&self, x: f32) -> f32 {
        match &self.lines {
            FieldBoundaryLines::One { line } => line.y(x),
            FieldBoundaryLines::Two {
                left_line,
                right_line,
                intersection,
            } => {
                if x > intersection.x {
                    right_line.y(x)
                } else {
                    left_line.y(x)
                }
            }
        }
    }

    /// Get the line segments that span the width of the image they are predicted from
    #[must_use]
    pub fn line_segments(&self) -> Vec<[(f32, f32); 2]> {
        match &self.lines {
            // One line segment, from the left edge to the right edge
            FieldBoundaryLines::One { line } => {
                let width = self.image.width() as f32;
                let y0 = line.y(0.0);
                let y1 = line.y(width);

                vec![[(0.0, y0), (width, y1)]]
            }
            // Two line segments, from the left edge to the intersection point and from the intersection point to the right edge
            FieldBoundaryLines::Two {
                left_line,
                right_line,
                intersection,
            } => {
                let width = self.image.width() as f32;
                let y0 = left_line.y(0.0);
                // This y is shared by both line segments as it is where they intersect
                let y1 = left_line.y(intersection.x);
                let y2 = right_line.y(width);

                vec![
                    [(0.0, y0), (intersection.x, y1)],
                    [(intersection.x, y1), (width, y2)],
                ]
            }
        }
    }
}

/// System that sets up the entities paths in rerun.
///
/// # Note
///
/// By logging a static [`rerun::Color`] component, we can avoid logging the color component
/// for each ball proposal and classification.
fn setup_boundary_debug_logging(dbg: DebugContext) {
    dbg.log_static(
        Top::make_entity_image_path("boundary/points"),
        &rerun::Color::from_rgb(255, 0, 255),
    );

    dbg.log_static(
        Top::make_entity_image_path("boundary/segments"),
        &rerun::Color::from_rgb(128, 0, 128),
    );
}

fn log_boundary_points(dbg: DebugContext, image: Res<Image<Top>>, boundary: Res<FieldBoundary>) {
    let points = boundary
        .points
        .iter()
        .map(|point| (point.x, point.y))
        .collect::<Vec<_>>();

    dbg.log_with_cycle(
        Top::make_entity_image_path("boundary/points"),
        image.cycle(),
        &rerun::Points2D::new(&points),
    );

    let line_segments = boundary.line_segments();
    dbg.log_with_cycle(
        Top::make_entity_image_path("boundary/segments"),
        image.cycle(),
        &rerun::LineStrips2D::new(&line_segments),
    );
}

pub fn init_field_boundary(mut commands: Commands, image: Res<Image<Top>>) {
    commands.insert_resource(FieldBoundary {
        lines: FieldBoundaryLines::One {
            line: Line {
                slope: 0.0,
                intercept: 0.0,
            },
        },
        error: 0.0,
        points: Vec::new(),
        image: image.clone(),
    });
}

pub fn detect_field_boundary(
    mut commands: Commands,
    mut model: ResMut<ModelExecutor<FieldBoundaryModel>>,
    image: Res<Image<Top>>,
) {
    // horizontal gap between predicted points relative to the original image
    let yuyv_image = image.clone();
    let gap = image.yuyv_image().width() / MODEL_INPUT_WIDTH as usize;
    let height = image.yuyv_image().height();
    let resized_image = image
        .resize(MODEL_INPUT_WIDTH, MODEL_INPUT_HEIGHT)
        .expect("Failed to resize image")
        .into_iter()
        // TODO: Retrain the model in u8 inputs
        .map(f32::from)
        .collect::<Vec<_>>();

    commands
        .infer_model(&mut model)
        .with_input(&resized_image)
        .create_resource()
        .spawn(move |result| {
            // Get the predicted points from the model output
            let points = result
                .chunks(2)
                .enumerate()
                // Map the x/y values back to their place in the original image
                .map(|(i, chunk)| FieldBoundaryPoint {
                    x: (i * gap) as f32,
                    y: chunk[0] * height as f32,
                    error: chunk[1],
                })
                .collect::<Vec<_>>();

            Some(fit_model(points, 2, yuyv_image))
        });
}

/// A model implementing the network from B-Human their [Deep Field Boundary](https://b-human.de/downloads/publications/2022/DeepFieldBoundary.pdf) paper
pub struct FieldBoundaryModel;

impl MlModel for FieldBoundaryModel {
    type Inputs = Vec<f32>;
    type Outputs = Vec<f32>;

    const ONNX_PATH: &'static str = "models/field_boundary.onnx";
}

/// A 2d line with a slope and intercept
#[derive(Debug, Clone)]
pub struct Line {
    pub slope: f32,
    pub intercept: f32,
}

impl Line {
    #[must_use]
    pub fn y(&self, x: f32) -> f32 {
        self.slope * x + self.intercept
    }

    #[must_use]
    pub fn intersection_point(&self, other: &Line) -> Point2<f32> {
        let x = (other.intercept - self.intercept) / (self.slope - other.slope);
        let y = self.slope * x + self.intercept;

        Point2::new(x, y)
    }
}

/// Line fiting algorithm as described in B-Human their paper
/// See <https://b-human.de/downloads/publications/2022/DeepFieldBoundary.pdf>
fn fit_line(spots: &[FieldBoundaryPoint]) -> (Line, f32) {
    let n_spots = spots.len();

    let mut a = nalgebra::DMatrix::<f32>::zeros(n_spots, 2);
    let mut y = nalgebra::DVector::<f32>::zeros(n_spots);

    for (i, spot) in spots.iter().enumerate() {
        let omega = spot.error * spot.error;
        a[(i, 0)] = omega;
        a[(i, 1)] = omega * spot.x;

        y[i] = omega * spot.y;
    }

    // use same epsilon as the numpy implementation
    let epsilon = f32::EPSILON * a.nrows().max(a.ncols()) as f32;

    // fit using least squares
    let Lstsq {
        solution,
        residuals,
        ..
    } = lstsq::lstsq(&a, &y, epsilon).unwrap();

    let line = Line {
        slope: solution[1],
        intercept: solution[0],
    };

    (line, residuals)
}

/// Model fitting algorithm as described in B-Human their paper
/// See <https://b-human.de/downloads/publications/2022/DeepFieldBoundary.pdf>
fn fit_model(points: Vec<FieldBoundaryPoint>, step: usize, image: Image<Top>) -> FieldBoundary {
    let width = image.width() as f32;

    // Get initial boundary fit based on single line
    let mut boundary = {
        let (line, error) = fit_line(&points);

        FieldBoundary {
            lines: FieldBoundaryLines::One { line },
            error,
            points,
            image,
        }
    };

    let n_points = boundary.points.len();

    for i in (2..n_points - 2).step_by(step) {
        let (left_line, left_error) = fit_line(&boundary.points[..i]);
        let (right_line, right_error) = fit_line(&boundary.points[i..]);

        let total_error = left_error + right_error;

        if total_error < boundary.error {
            let intersection = left_line.intersection_point(&right_line);

            // make sure intersection is within the image
            if intersection.x < 0.0 || intersection.x > width {
                continue;
            }

            // update with better fitted boundary
            boundary = FieldBoundary {
                lines: FieldBoundaryLines::Two {
                    left_line,
                    right_line,
                    intersection,
                },
                error: total_error,
                points: boundary.points,
                image: boundary.image,
            };
        }
    }

    boundary
}
