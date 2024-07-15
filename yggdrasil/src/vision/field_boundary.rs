//! Module for detecting the field boundary lines from the top camera image
//!

use std::{num::NonZeroU32, ops::Deref};

use crate::{
    core::debug::DebugContext,
    core::ml::{MlModel, MlTask, MlTaskResource},
    prelude::*,
    vision::camera::{self, Image, TopImage},
};
use fast_image_resize as fr;
use heimdall::YuyvImage;
use lstsq::Lstsq;
use nalgebra::Point2;
use nidhogg::types::color;

const MODEL_INPUT_WIDTH: u32 = 40;
const MODEL_INPUT_HEIGHT: u32 = 30;

/// Module for detecting the field boundary lines from the top camera image
///
/// It adds the following resources to the app:
/// - [`FieldBoundary`]
pub struct FieldBoundaryModule;

impl Module for FieldBoundaryModule {
    fn initialize(self, app: App) -> Result<App> {
        Ok(app
            .add_ml_task::<FieldBoundaryModel>()?
            .add_startup_system(init_field_boundary)?
            .add_system_chain((
                detect_field_boundary.after(camera::camera_system),
                log_boundary_points,
            )))
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
#[derive(Clone)]
pub struct FieldBoundary {
    /// The fitted field boundary lines
    pub lines: FieldBoundaryLines,
    /// Sum of squared residual error from fitting the boundary
    error: f32,
    /// The predicted points used to fit the boundary
    points: Vec<FieldBoundaryPoint>,
    /// The image the boundary was predicted from
    pub image: Image,
}

impl FieldBoundary {
    /// Get the height of the boundary at a given horizontal pixel
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
    pub fn line_segments(&self) -> Vec<[(f32, f32); 2]> {
        match &self.lines {
            // One line segment, from the left edge to the right edge
            FieldBoundaryLines::One { line } => {
                let width = self.image.yuyv_image().width() as f32;
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
                let width = self.image.yuyv_image().width() as f32;
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

#[system]
fn log_boundary_points(
    dbg: &DebugContext,
    image: &TopImage,
    boundary: &FieldBoundary,
) -> Result<()> {
    let points = boundary
        .points
        .iter()
        .map(|point| (point.x, point.y))
        .collect::<Vec<_>>();

    dbg.log_points2d_for_image(
        "top_camera/image/boundary_points",
        &points,
        image,
        color::u8::MAGENTA,
    )?;

    let line_segments = boundary.line_segments();

    dbg.log_lines2d_for_image(
        "top_camera/image/boundary_line_segments",
        &line_segments,
        image,
        color::u8::PURPLE,
    )?;

    Ok(())
}

/// For keeping track of the image that a field boundary is detected from
struct FieldBoundaryImage(Image);

#[system]
fn detect_field_boundary(
    model: &mut MlTask<FieldBoundaryModel>,
    field_boundary_image: &mut FieldBoundaryImage,
    boundary: &mut FieldBoundary,
    top_image: &TopImage,
) -> Result<()> {
    // Start a new inference if the image has changed
    // TODO: Some kind of callback/event system would be nice to avoid doing the timestamp comparison everywhere
    if field_boundary_image.0.timestamp() != top_image.timestamp() && !model.active() {
        let resized_image = resize_yuyv(top_image.yuyv_image());
        if let Ok(()) = model.try_start_infer(&resized_image) {
            // We need to keep track of the image we started the inference with
            //
            // TODO: We should find a better way to do this bundling of mltask + metadata
            *field_boundary_image = FieldBoundaryImage(top_image.deref().clone());
        };
    }

    // Otherwise, poll the model for the result
    if let Some(result) = model.poll::<Vec<f32>>().transpose()? {
        // horizontal gap between predicted points relative to the original image
        let gap = top_image.yuyv_image().width() / MODEL_INPUT_WIDTH as usize;
        let height = top_image.yuyv_image().height();

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

        // Get the image we set when we started inference
        let image = field_boundary_image.0.clone();

        *boundary = fit_model(points, 2, image)?;
    }

    Ok(())
}

// Resize yuyv image to correct input shape
fn resize_yuyv(yuyv_image: &YuyvImage) -> Vec<f32> {
    let src_image = fr::Image::from_vec_u8(
        NonZeroU32::new((yuyv_image.width() / 2) as u32).unwrap(),
        NonZeroU32::new(yuyv_image.height() as u32).unwrap(),
        yuyv_image.to_vec(),
        fr::PixelType::U8x4,
    )
    .expect("Failed to create image for resizing");

    // Resize the image to the correct input shape for the model
    let mut dst_image = fr::Image::new(
        NonZeroU32::new(MODEL_INPUT_WIDTH).unwrap(),
        NonZeroU32::new(MODEL_INPUT_HEIGHT).unwrap(),
        src_image.pixel_type(),
    );

    let mut resizer = fr::Resizer::new(fr::ResizeAlg::Nearest);
    resizer
        .resize(&src_image.view(), &mut dst_image.view_mut())
        .expect("Failed to resize image");

    // Remove every second y value from the yuyv image
    dst_image
        .buffer()
        .iter()
        .copied()
        .enumerate()
        .filter(|(i, _)| (i + 2) % 4 != 0)
        .map(|(_, p)| p as f32)
        .collect()
}

/// A model implementing the network from B-Human their [Deep Field Boundary](https://b-human.de/downloads/publications/2022/DeepFieldBoundary.pdf) paper
pub struct FieldBoundaryModel;

impl MlModel for FieldBoundaryModel {
    type InputType = f32;
    type OutputType = f32;
    const ONNX_PATH: &'static str = "models/field_boundary.onnx";
}

/// A 2d line with a slope and intercept
#[derive(Debug, Clone)]
pub struct Line {
    pub slope: f32,
    pub intercept: f32,
}

impl Line {
    pub fn y(&self, x: f32) -> f32 {
        self.slope * x + self.intercept
    }

    pub fn intersection_point(&self, other: &Line) -> Point2<f32> {
        let x = (other.intercept - self.intercept) / (self.slope - other.slope);
        let y = self.slope * x + self.intercept;

        Point2::new(x, y)
    }
}

/// Line fiting algorithm as described in B-Human their paper
/// See <https://b-human.de/downloads/publications/2022/DeepFieldBoundary.pdf>
fn fit_line(spots: &[FieldBoundaryPoint]) -> Result<(Line, f32)> {
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

    Ok((line, residuals))
}

/// Model fitting algorithm as described in B-Human their paper
/// See <https://b-human.de/downloads/publications/2022/DeepFieldBoundary.pdf>
fn fit_model(points: Vec<FieldBoundaryPoint>, step: usize, image: Image) -> Result<FieldBoundary> {
    let width = image.yuyv_image().width() as f32;

    // Get initial boundary fit based on single line
    let mut boundary = {
        let (line, error) = fit_line(&points)?;

        FieldBoundary {
            lines: FieldBoundaryLines::One { line },
            error,
            points,
            image,
        }
    };

    let n_points = boundary.points.len();

    for i in (2..n_points - 2).step_by(step) {
        let (left_line, left_error) = fit_line(&boundary.points[..i])?;
        let (right_line, right_error) = fit_line(&boundary.points[i..])?;

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

    Ok(boundary)
}

#[startup_system]
fn init_field_boundary(storage: &mut Storage, top_image: &TopImage) -> Result<()> {
    let field_boundary_image = FieldBoundaryImage(top_image.deref().clone());

    // Initialize the field boundary with a single line at the top of the image
    let boundary = FieldBoundary {
        lines: FieldBoundaryLines::One {
            line: Line {
                slope: 0.0,
                intercept: 0.0,
            },
        },
        error: 0.0,
        points: Vec::new(),
        image: top_image.deref().clone(),
    };

    storage.add_resource(Resource::new(field_boundary_image))?;
    storage.add_resource(Resource::new(boundary))?;

    Ok(())
}
