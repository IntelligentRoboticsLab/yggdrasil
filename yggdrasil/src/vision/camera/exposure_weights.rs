use crate::vision::field_boundary::FieldBoundary;

use super::Camera;
use bevy::{prelude::*, tasks::IoTaskPool};
use heimdall::{ExposureWeights, Top};
use tasks::conditions::task_finished;

const SAMPLES_PER_COLUMN: usize = 4;
const ABOVE_FIELD_WEIGHT: u8 = 0;
const BELOW_FIELD_WEIGHT: u8 = 15;
const MIN_BOTTOM_ROW_WEIGHT: u8 = 10;
const WEIGHT_SLOPE: f32 = (BELOW_FIELD_WEIGHT - ABOVE_FIELD_WEIGHT) as f32;

pub(crate) struct ExposureWeightsPlugin;

impl Plugin for ExposureWeightsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (update_exposure_weights, sync_exposure_weights)
                .chain()
                .run_if(task_finished::<FieldBoundary>),
        );
    }
}

fn update_exposure_weights(
    mut exposure_weights: ResMut<ExposureWeights>,
    field_boundary: Res<FieldBoundary>,
) {
    let (width, height) = exposure_weights.top.window_size();
    let (column_width, row_height) = (width / 4, height / 4);

    let mut weights = [0; 16];

    for column_index in 0..4 {
        let column_start = column_index * column_width;
        let column_end = column_start + column_width;

        let samples = (column_start..column_end)
            .step_by(column_width as usize / SAMPLES_PER_COLUMN)
            .map(|x| field_boundary.height_at_pixel(x as f32));

        let n = samples.len() as f32;
        let field_height = (samples.sum::<f32>() / n) as u32;

        for row_index in 0..4 {
            let row_start = row_index * row_height;
            let row_end = row_start + row_height;

            let weight_index = row_index * 4 + column_index;

            weights[weight_index as usize] = if row_end < field_height {
                ABOVE_FIELD_WEIGHT
            } else if row_start > field_height {
                BELOW_FIELD_WEIGHT
            } else {
                let fract = (field_height - row_start) as f32 / row_height as f32;

                ((f32::from(ABOVE_FIELD_WEIGHT) + WEIGHT_SLOPE * fract) as u8)
                    .clamp(ABOVE_FIELD_WEIGHT, BELOW_FIELD_WEIGHT)
            }
        }
    }

    for weight in weights.iter_mut().skip(12) {
        *weight = (*weight).max(MIN_BOTTOM_ROW_WEIGHT);
    }

    exposure_weights.top.update(weights);
}

fn sync_exposure_weights(
    exposure_weights: Res<ExposureWeights>,
    top_camera: Res<Camera<Top>>,
    bottom_camera: Res<Camera<Top>>,
) {
    let exposure_weights = exposure_weights.clone();
    let top_camera = top_camera.inner.clone();
    let bottom_camera = bottom_camera.inner.clone();

    let pool = IoTaskPool::get();

    pool.spawn(async move {
        if let Ok(top_camera) = top_camera.lock() {
            top_camera
                .camera_device()
                .set_auto_exposure_weights(&exposure_weights.top)
                .expect("failed to set auto exposure weights for top camera");
        }

        if let Ok(bottom_camera) = bottom_camera.lock() {
            bottom_camera
                .camera_device()
                .set_auto_exposure_weights(&exposure_weights.bottom)
                .expect("failed to set auto exposure weights for bottom camera");
        }
    })
    .detach();
}
