use core::f32;
use std::iter::IntoIterator;

use bevy::prelude::*;

use filter::{
    CovarianceMatrix, StateMatrix, StateTransform, StateVector, UnscentedKalmanFilter, WeightVector,
};
use nalgebra::{self as na, point, vector, ComplexField};
use rerun::components::RotationAxisAngle;

use crate::{
    core::{
        config::{
            layout::{Direction, FieldLine, LayoutConfig},
            showtime::PlayerConfig,
        },
        debug::DebugContext,
    },
    motion::odometry::Odometry,
    nao::Cycle,
    vision::line_detection::line::{Line2, LineSegment2},
};

use super::correspondence::LineCorrespondences;

pub struct PoseFilterPlugin;

impl Plugin for PoseFilterPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostStartup, initialize_particles)
            .add_systems(Update, (odometry_update, line_update, log_single).chain());
    }
}

#[derive(Debug, Clone, Copy)]
pub struct RobotPose {
    pub inner: na::Isometry2<f32>,
}

impl RobotPose {
    pub fn position(&self) -> na::Point2<f32> {
        point![self.inner.translation.x, self.inner.translation.y]
    }

    pub fn angle(&self) -> f32 {
        self.inner.rotation.angle()
    }
}

impl From<RobotPose> for StateVector<3> {
    fn from(pose: RobotPose) -> Self {
        let translation = pose.inner.translation.vector;
        let rotation = pose.inner.rotation;
        translation.xy().push(rotation.angle())
    }
}

impl From<StateVector<3>> for RobotPose {
    fn from(state: StateVector<3>) -> Self {
        Self {
            inner: na::Isometry2::new(state.xy(), state.z),
        }
    }
}

// In order to handle non-linear values (angles), we need a custom weighted mean and residual calculation
impl StateTransform<3> for RobotPose {
    fn into_state_mean<const N: usize>(
        weights: WeightVector<N>,
        states: StateMatrix<3, N>,
    ) -> StateVector<3> {
        let mut mean_translation = na::SVector::zeros();
        let mut mean_angle = na::Complex::ZERO;

        for (&weight, pose) in weights.iter().zip(states.column_iter()) {
            mean_translation += weight * pose.xy();
            mean_angle += weight * na::Complex::cis(pose.z);
        }

        mean_translation.xy().push(mean_angle.argument())
    }

    fn residual(measurement: StateVector<3>, prediction: StateVector<3>) -> StateVector<3> {
        (measurement.xy() - prediction.xy()).push(
            (na::UnitComplex::new(measurement.z) / na::UnitComplex::new(prediction.z)).angle(),
        )
    }
}

#[derive(Clone, Component, Deref, DerefMut)]
pub struct RobotPoseFilter(UnscentedKalmanFilter<3, 7, RobotPose>);

impl RobotPoseFilter {
    pub fn fix_covariance(&mut self) {
        let cov = self.covariance_mut();
        *cov = (cov.clone() + cov.transpose()) / 2.0;
    }
}

fn initialize_particles(
    mut commands: Commands,
    layout: Res<LayoutConfig>,
    player: Res<PlayerConfig>,
) {
    for particle in initial_particles(&layout, player.player_number) {
        commands.spawn(particle);
    }
}

fn odometry_update(odometry: Res<Odometry>, mut particles: Query<&mut RobotPoseFilter>) {
    for mut particle in &mut particles {
        particle
            .predict(
                |pose| RobotPose {
                    inner: pose.inner * odometry.offset_to_last,
                },
                CovarianceMatrix::from_diagonal_element(0.05),
            )
            .unwrap();
    }
}

fn line_update(
    dbg: DebugContext,
    correspondences: Query<(&Cycle, &LineCorrespondences), Added<LineCorrespondences>>,
    mut particles: Query<&mut RobotPoseFilter>,
) {
    for mut particle in &mut particles {
        for (cycle, new_correspondences) in correspondences.iter() {
            for correspondence in &new_correspondences.0 {
                // skip circles for now
                let FieldLine::Segment(field_line) = correspondence.field_line else {
                    continue;
                };

                let Some(direction) = correspondence.field_line.direction() else {
                    continue;
                };

                // the line that the robot actually saw
                let detected = match direction {
                    Direction::AlongX
                        if correspondence.detected_line.end.y
                            < correspondence.detected_line.start.y =>
                    {
                        LineSegment2::new(
                            correspondence.detected_line.end,
                            correspondence.detected_line.start,
                        )
                    }
                    Direction::AlongY
                        if correspondence.detected_line.end.x
                            < correspondence.detected_line.start.x =>
                    {
                        LineSegment2::new(
                            correspondence.detected_line.end,
                            correspondence.detected_line.start,
                        )
                    }
                    _ => correspondence.detected_line,
                };

                let projected = correspondence.projected_line;

                // TODO: remove and make validity check
                if projected.length() < 0.5 {
                    continue;
                }

                // let distance = match direction {
                //     Direction::AlongX => detected.center().y - projected.center().y,
                //     Direction::AlongY => detected.center().x - projected.center().x,
                // };

                // distance to current pose
                let pose = particle.state();

                // line from the robot to the detected line
                let relative_line = (correspondence.pose.inner.inverse() * detected).to_line();
                let relative_line_segm = correspondence.pose.inner.inverse() * detected;

                let orthogonal_projection = relative_line.project(point![0.0, 0.0]);

                let measured_angle = {
                    let mut angle = -f32::atan2(
                        orthogonal_projection.coords.y,
                        orthogonal_projection.coords.x,
                    );

                    angle = match direction {
                        Direction::AlongX => angle + std::f32::consts::FRAC_PI_2,
                        Direction::AlongY => angle,
                    };

                    normalize_angle(angle)
                };
                let measured_angle_alternative =
                    normalize_angle(measured_angle - std::f32::consts::PI);

                let measured_angle = if normalize_angle(measured_angle_alternative - pose.angle())
                    .abs()
                    < normalize_angle(measured_angle - pose.angle()).abs()
                {
                    measured_angle_alternative
                } else {
                    measured_angle
                };

                let c = measured_angle.cos();
                let s = measured_angle.sin();

                let angle_rotation_matrix = na::Matrix2::new(c, -s, s, c);

                // const Vector2f orthogonalProjection = angleRotationMatrix * Vector2f(line.orthogonalProjection.x(), line.orthogonalProjection.y());
                // println!("Rotation matrix: {}", angle_rotation_matrix);

                // println!("Orthogonal projection: {:?}", orthogonal_projection);

                let og_orthogonal_projection = orthogonal_projection.coords;

                let orthogonal_projection = angle_rotation_matrix
                    * na::Vector2::new(
                        orthogonal_projection.coords.x,
                        orthogonal_projection.coords.y,
                    );

                // println!("Rotated orthogonal projection: {:?}", orthogonal_projection);

                particle.fix_covariance();

                let measured = match direction {
                    Direction::AlongX => field_line.start.y - orthogonal_projection.y,
                    Direction::AlongY => field_line.start.x - orthogonal_projection.x,
                };

                // println!("Measured distance: {}, angle: {}", measured, measured_angle);

                let measurement = LineMeasurement {
                    distance: measured,
                    angle: measured_angle,
                };

                dbg.log_with_cycle(
                    "localization_shi",
                    *cycle,
                    &rerun::LineStrips3D::new([
                        [
                            (relative_line_segm.start.x, relative_line_segm.start.y, 0.0),
                            (0.0, 0.0, 0.0),
                        ],
                        [
                            (relative_line_segm.end.x, relative_line_segm.end.y, 0.0),
                            (0.0, 0.0, 0.0),
                        ],
                        [
                            (0.0, 0.0, 0.0),
                            (orthogonal_projection.x, orthogonal_projection.y, 0.0),
                        ],
                        [
                            (relative_line_segm.start.x, relative_line_segm.start.y, 0.0),
                            (relative_line_segm.end.x, relative_line_segm.end.y, 0.0),
                        ],
                    ])
                    .with_colors([(255, 255, 0), (255, 255, 0), (255, 0, 0), (0, 255, 0)])
                    .with_radii([0.02, 0.02, 0.05, 0.03])
                    .with_labels([
                        "",
                        &format!("{}", measured_angle),
                        "orthogonal proj",
                        "relative line",
                    ]),
                );

                // dbg.log_with_cycle(
                //     "localization_shi_unrotated",
                //     *cycle,
                //     &rerun::LineStrips3D::new([[
                //         (0.0, 0.0, 0.0),
                //         (
                //             (og_orthogonal_projection).x,
                //             (og_orthogonal_projection).y,
                //             0.0,
                //         ),
                //     ]])
                //     .with_colors([(0, 255, 0), (0, 255, 0)])
                //     .with_radii([0.1, 0.1]),
                // );

                // println!(
                //     "Updating particle in direction {:?} with {:?}",
                //     direction, measurement
                // );

                let mut cov_mat = CovarianceMatrix::from_diagonal_element(1.0);
                cov_mat *= angle_rotation_matrix * cov_mat * angle_rotation_matrix.transpose();

                // 4.f * yVariance / (line.perceptStart - line.perceptEnd).squaredNorm()
                // const float angleVariance = sqr(std::atan(std::sqrt()));

                let cov_mat = match direction {
                    Direction::AlongX => {
                        let y_variance = cov_mat[(1, 1)];
                        let angle_variance = (4.0 * y_variance / (detected.length().powi(2)))
                            .sqrt()
                            .atan()
                            .powi(2);

                        CovarianceMatrix::<2>::new(y_variance, 0.0, 0.0, angle_variance)
                    }
                    Direction::AlongY => {
                        let x_variance = cov_mat[(0, 0)];
                        let angle_variance = (4.0 * x_variance / (detected.length().powi(2)))
                            .sqrt()
                            .atan()
                            .powi(2);

                        CovarianceMatrix::<2>::new(x_variance, 0.0, 0.0, angle_variance)
                    }
                };

                // update the particle with the difference
                particle
                    .update(
                        |pose| {
                            let state: StateVector<3> = pose.into();

                            match direction {
                                Direction::AlongX => LineMeasurement {
                                    distance: state.y,
                                    angle: state.z,
                                },
                                Direction::AlongY => LineMeasurement {
                                    distance: state.x,
                                    angle: state.z,
                                },
                            }
                        },
                        measurement,
                        cov_mat,
                    )
                    .unwrap();
            }
        }
    }

    let mut weighted_state = StateVector::<3>::zeros();
    let mut total_weight = 0.0;

    for particle in &particles {
        // Weight inversely to covariance sum - particles with lower covariance get higher weight
        let weight = 1.0 / (particle.0.covariance().sum() + 1e-10);
        weighted_state += weight * particle.0.state;
        total_weight += weight;
    }

    // Normalize
    weighted_state /= total_weight;

    // Update all particles with the weighted state
    for mut particle in &mut particles {
        particle.0.state = weighted_state;
    }
}

fn log_single(dbg: DebugContext, cycle: Res<Cycle>, particles: Query<&RobotPoseFilter>) {
    if let Some(a) = particles.iter().next() {
        let pos = a.state();

        dbg.log_with_cycle(
            "new_pose",
            *cycle,
            &rerun::Boxes3D::from_centers_and_half_sizes([(0.0, 0.0, 0.0)], [(0.1, 0.1, 0.1)])
                .with_colors([(255, 0, 0)]),
        );

        dbg.log_with_cycle(
            "new_pose",
            *cycle,
            &rerun::Transform3D::from_translation_rotation(
                (pos.inner.translation.x, pos.inner.translation.y, 0.0),
                rerun::Rotation3D::AxisAngle(RotationAxisAngle::new(
                    (0.0, 0.0, 1.0),
                    pos.inner.rotation.angle(),
                )),
            )
            .with_axis_length(0.25),
        );
    };
}

fn initial_particles(
    layout: &LayoutConfig,
    player_num: u8,
) -> impl IntoIterator<Item = RobotPoseFilter> {
    let position = RobotPose {
        inner: layout.initial_positions.player(player_num).isometry,
    };

    (0..10).map(move |_| {
        RobotPoseFilter(UnscentedKalmanFilter::new(
            position,
            CovarianceMatrix::from_diagonal_element(0.1),
        ))
    })
}

#[derive(Debug, Clone, Copy)]
pub struct LineMeasurement {
    pub distance: f32,
    pub angle: f32,
}

impl From<StateVector<2>> for LineMeasurement {
    fn from(state: StateVector<2>) -> Self {
        Self {
            distance: state.x,
            angle: state.y,
        }
    }
}

impl From<LineMeasurement> for StateVector<2> {
    fn from(segment: LineMeasurement) -> Self {
        vector![segment.distance, segment.angle]
    }
}

impl StateTransform<2> for LineMeasurement {
    fn into_state_mean<const N: usize>(
        weights: nalgebra::SVector<filter::Weight, N>,
        states: nalgebra::SMatrix<f32, 2, N>,
    ) -> StateVector<2> {
        let mut mean_distance = 0.0;
        let mut mean_angle = na::Complex::ZERO;

        for (&weight, pose) in weights.iter().zip(states.column_iter()) {
            mean_distance += weight * pose.x;
            mean_angle += weight * na::Complex::cis(pose.y);
        }

        vector![mean_distance, mean_angle.argument()]
    }

    fn residual(measurement: StateVector<2>, prediction: StateVector<2>) -> StateVector<2> {
        vector![
            measurement.x - prediction.x,
            (na::UnitComplex::new(measurement.y) / na::UnitComplex::new(prediction.y)).angle()
        ]
    }
}

// normalize an angle to be in the range [-PI, PI]
pub fn normalize_angle(mut angle: f32) -> f32 {
    use std::f32::consts::{PI, TAU};
    angle %= TAU;
    if angle > PI {
        angle -= TAU;
    } else if angle < -PI {
        angle += TAU;
    }
    angle
}
