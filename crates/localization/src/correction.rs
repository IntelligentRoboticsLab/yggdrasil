use nalgebra::{matrix, vector, Isometry2, Matrix2, Vector2};
use serde::{Deserialize, Serialize};

use vision::line_detection::line::LineSegment2;
use yggdrasil_config::layout::LayoutConfig;

use super::{
    correspondence::{correspond_field_lines, FieldLineCorrespondence, PointCorrespondence},
    LocalizationConfig,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GradientDescentConfig {
    /// Threshold at which the fit is considered converged
    pub convergence_threshold: f32,
    /// Step size for the gradient descent
    pub step_size: f32,
    /// Maximum number of correction iterations
    pub max_correction_iters: usize,
    /// Maximum number of refitting iterations
    pub max_refit_iters: usize,
}

#[must_use]
pub fn fit_field_lines(
    lines: &[LineSegment2],
    cfg: &LocalizationConfig,
    layout: &LayoutConfig,
) -> Option<(Vec<FieldLineCorrespondence>, f32)> {
    let mut correction = Isometry2::identity();

    for _ in 0..cfg.gradient_descent.max_refit_iters {
        let point_correspondences =
            get_point_correspondences(&correspond_field_lines(lines, cfg, layout, correction));

        if point_correspondences.is_empty() {
            return None;
        }

        let weight_matrices = get_weight_matrices(&point_correspondences, correction);

        for _ in 0..cfg.gradient_descent.max_correction_iters {
            let translation_gradient = point_correspondences
                .iter()
                .zip(weight_matrices.iter())
                .map(|(point_correspondences, weight_matrix)| {
                    2.0 * weight_matrix
                        * ((correction * point_correspondences.measurement)
                            - point_correspondences.reference)
                })
                .sum::<Vector2<f32>>()
                / point_correspondences.len() as f32;

            let rotation = correction.rotation.angle();
            let rotation_derivative = matrix![-rotation.sin(), -rotation.cos();
                                              rotation.cos(), -rotation.sin()];

            let rotation_gradient = point_correspondences
                .iter()
                .zip(weight_matrices.iter())
                .map(|(point_correspondences, weight_matrix)| {
                    (2.0 * point_correspondences.measurement.coords.transpose()
                        * rotation_derivative.transpose()
                        * weight_matrix
                        * ((correction * point_correspondences.measurement)
                            - point_correspondences.reference))
                        .x
                })
                .sum::<f32>()
                / point_correspondences.len() as f32;

            correction = nalgebra::Isometry2::new(
                correction.translation.vector
                    - (cfg.gradient_descent.step_size * translation_gradient),
                rotation - cfg.gradient_descent.step_size * rotation_gradient,
            );

            let gradient_norm = vector![
                translation_gradient.x,
                translation_gradient.y,
                rotation_gradient
            ]
            .norm();

            if gradient_norm < cfg.gradient_descent.convergence_threshold {
                break;
            }
        }
    }

    let field_line_correspondences = correspond_field_lines(lines, cfg, layout, correction);
    let point_correspondences = get_point_correspondences(&field_line_correspondences);
    let weight_matrices = get_weight_matrices(&point_correspondences, correction);
    let fit_error = get_fit_error(&point_correspondences, &weight_matrices, correction);

    Some((field_line_correspondences, fit_error))
}

fn get_point_correspondences(
    field_line_correspondences: &[FieldLineCorrespondence],
) -> Vec<PointCorrespondence> {
    field_line_correspondences
        .iter()
        .flat_map(|field_line_correspondence| {
            [
                field_line_correspondence.start,
                field_line_correspondence.end,
            ]
        })
        .collect()
}

fn get_weight_matrices(
    point_correspondences: &[PointCorrespondence],
    correction: Isometry2<f32>,
) -> Vec<Matrix2<f32>> {
    point_correspondences
        .iter()
        .map(|point| {
            let normal = (correction * point.measurement) - point.reference;

            if let Some(norm) = normal.try_normalize(f32::EPSILON) {
                norm * norm.transpose()
            } else {
                Matrix2::zeros()
            }
        })
        .collect::<Vec<_>>()
}

fn get_fit_error(
    point_correspondences: &[PointCorrespondence],
    weight_matrices: &[Matrix2<f32>],
    correction: nalgebra::Isometry2<f32>,
) -> f32 {
    point_correspondences
        .iter()
        .zip(weight_matrices.iter())
        .map(|(point_correspondences, weight_matrix)| {
            ((correction * point_correspondences.measurement - point_correspondences.reference)
                .transpose()
                * weight_matrix
                * (correction * point_correspondences.measurement
                    - point_correspondences.reference))
                .x
        })
        .sum::<f32>()
        / point_correspondences.len() as f32
}
