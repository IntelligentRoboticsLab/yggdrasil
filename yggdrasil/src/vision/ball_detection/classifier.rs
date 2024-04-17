use std::num::NonZeroU32;
use std::ops::Deref;
use std::time::Instant;

use fast_image_resize as fr;

use nalgebra::{Point2, Point3};

use crate::camera::{Image, TopImage};
use crate::debug::DebugContext;
use crate::prelude::*;

use crate::ml::{MlModel, MlTask, MlTaskResource};

use super::proposal::BallProposals;

pub(crate) struct BallClassifierModule;

impl Module for BallClassifierModule {
    fn initialize(self, app: App) -> Result<App> {
        app.add_system(detect_balls.after(super::proposal::get_proposals))
            .add_startup_system(init_ball_classifier)?
            .add_ml_task::<BallClassifierModel>()
    }
}

#[startup_system]
fn init_ball_classifier(storage: &mut Storage, top_image: &TopImage) -> Result<()> {
    // Initialize the field boundary with a single line at the top of the image
    let balls = Balls {
        balls: Vec::new(),
        image: top_image.deref().clone(),
    };

    storage.add_resource(Resource::new(balls))?;

    Ok(())
}

struct BallClassifierModel;

impl MlModel for BallClassifierModel {
    const ONNX_PATH: &'static str = "models/ball_classifier_non_quant.onnx";
    type InputType = f32;
    type OutputType = f32;
}

#[derive(Debug, Clone, Default)]
pub struct Ball {
    pub position_image: Point2<f32>,
    pub position_world: Point3<f32>,
    pub distance: f32,
}

#[derive(Clone)]
pub struct Balls {
    pub balls: Vec<Ball>,
    pub image: Image,
}

#[system]
fn detect_balls(
    proposals: &BallProposals,
    model: &mut MlTask<BallClassifierModel>,
    balls: &mut Balls,
    ctx: &DebugContext,
) -> Result<()> {
    if balls.image.timestamp() == proposals.image.timestamp() {
        return Ok(());
    }

    for proposal in proposals.proposals.iter() {
        // get patch

        let size = 140 / proposal.distance_to_ball as usize;
        let patch = proposals.image.get_grayscale_patch(
            (proposal.position.x, proposal.position.y),
            size,
            size,
        );

        let patch = resize_patch(size, size, patch);

        let start = Instant::now();
        if let Ok(()) = model.try_start_infer(&patch) {
            loop {
                if let Ok(Some(result)) = model.poll::<Vec<f32>>().transpose() {
                    tracing::info!("inference took: {:?}", start.elapsed());
                    let confidence = result[0];
                    if confidence > 0.5 {
                        ctx.log_boxes2d_with_class(
                            "/top_camera/image/real_balls",
                            &[(proposal.position.x as f32, proposal.position.y as f32)],
                            &[(16.0, 16.0)],
                            vec!["ball".to_string()],
                            proposals.image.cycle(),
                        )?;
                    }

                    break;
                }
            }
        }
    }

    balls.image = proposals.image.clone();

    Ok(())
}

/// Resize yuyv image to correct input shape
fn resize_patch(width: usize, height: usize, patch: Vec<u8>) -> Vec<f32> {
    let src_image = fr::Image::from_vec_u8(
        NonZeroU32::new(width as u32).unwrap(),
        NonZeroU32::new(height as u32).unwrap(),
        patch,
        fr::PixelType::U8,
    )
    .expect("Failed to create image for resizing");

    // Resize the image to the correct input shape for the model
    let mut dst_image = fr::Image::new(
        NonZeroU32::new(32).unwrap(),
        NonZeroU32::new(32).unwrap(),
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
        .map(|p| *p as f32 / 255.0)
        .collect()
}
