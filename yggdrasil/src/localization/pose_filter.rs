use std::iter::IntoIterator;

use bevy::prelude::*;

use filter::{
    CovarianceMatrix, StateMatrix, StateTransform, StateVector, UnscentedKalmanFilter, WeightVector,
};
use nalgebra::{self as na, point, vector, ComplexField};

use crate::{
    core::{
        config::{
            layout::{FieldLine, LayoutConfig, ParallelAxis},
            showtime::PlayerConfig,
        },
        debug::DebugContext,
    },
    localization::correspondence::LineCorrespondences,
    motion::odometry::Odometry,
    nao::Cycle,
};

const PARTICLE_SCORE_DECAY: f32 = 0.9;
const PARTICLE_SCORE_DEFAULT: f32 = 10.0;
const PARTICLE_SCORE_INCREASE: f32 = 0.5;
const PARTICLE_SCORE_BONUS: f32 = 2.0;
const PARTICLE_BONUS_THRESHOLD: f32 = 0.5;
const PARTICLE_RETAIN_FACTOR: f32 = 0.5;

pub struct PoseFilterPlugin;

impl Plugin for PoseFilterPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostStartup, initialize_particles_and_pose)
            .add_systems(
                Update,
                (
                    odometry_update,
                    line_update,
                    filter_particles,
                    resample,
                    sensor_resetting,
                    log_particles,
                )
                    .chain(),
            );
    }
}

#[derive(Resource, Default, Debug, Clone, Copy)]
pub struct RobotPose {
    pub inner: na::Isometry2<f32>,
}

impl RobotPose {
    pub fn new(inner: na::Isometry2<f32>) -> Self {
        Self { inner }
    }

    /// Position of the robot in the field
    #[must_use]
    pub fn position(&self) -> na::Point2<f32> {
        point![self.inner.translation.x, self.inner.translation.y]
    }

    /// Angle of the robot in radians in the range [-pi, pi]
    #[must_use]
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
pub struct RobotPoseFilter {
    #[deref]
    filter: UnscentedKalmanFilter<3, 7, RobotPose>,
    score: f32,
}

impl RobotPoseFilter {
    // Mitigate numerical instability with covariance matrix by ensuring it is symmetric
    fn fix_covariance(&mut self) {
        let cov = &mut self.filter.covariance;
        *cov = (*cov + cov.transpose()) * 0.5;
    }
}

fn initialize_particles_and_pose(
    mut commands: Commands,
    layout: Res<LayoutConfig>,
    player: Res<PlayerConfig>,
) {
    for particle in initial_particles(&layout, player.player_number) {
        commands.spawn(particle);
    }

    commands.insert_resource(RobotPose::new(
        layout
            .initial_positions
            .player(player.player_number)
            .isometry,
    ))
}

fn odometry_update(odometry: Res<Odometry>, mut particles: Query<&mut RobotPoseFilter>) {
    for mut particle in &mut particles {
        particle
            .predict(
                |pose| RobotPose {
                    inner: pose.inner * odometry.offset_to_last,
                },
                CovarianceMatrix::from_diagonal(&na::Vector3::new(0.05, 0.05, 0.01)),
            )
            .unwrap();

        particle.score *= PARTICLE_SCORE_DECAY;
    }
}

fn line_update(
    added_correspondences: Query<&LineCorrespondences, Added<LineCorrespondences>>,
    mut particles: Query<&mut RobotPoseFilter>,
) {
    for mut particle in &mut particles {
        for correspondences in &added_correspondences {
            for correspondence in correspondences.iter() {
                // skip circles for now
                let FieldLine::Segment {
                    segment: field_line,
                    axis,
                } = correspondence.field_line
                else {
                    continue;
                };

                let current_pose = particle.prediction();

                // line from the robot to the detected line
                let relative_line =
                    (correspondence.pose.inner.inverse() * correspondence.detected_line).to_line();

                let orthogonal_projection = relative_line.project(point![0.0, 0.0]);

                let measured_angle = {
                    let mut angle = -f32::atan2(
                        orthogonal_projection.coords.y,
                        orthogonal_projection.coords.x,
                    );

                    angle = match axis {
                        ParallelAxis::X => angle + std::f32::consts::FRAC_PI_2,
                        ParallelAxis::Y => angle,
                    };

                    normalize_angle(angle)
                };
                let measured_angle_alternative =
                    normalize_angle(measured_angle - std::f32::consts::PI);

                let measured_angle =
                    if normalize_angle(measured_angle_alternative - current_pose.angle()).abs()
                        < normalize_angle(measured_angle - current_pose.angle()).abs()
                    {
                        measured_angle_alternative
                    } else {
                        measured_angle
                    };

                let c = measured_angle.cos();
                let s = measured_angle.sin();

                let rotation = na::Matrix2::new(c, -s, s, c);

                let rotated_projection = rotation * orthogonal_projection.coords;

                particle.fix_covariance();

                let measured = match axis {
                    ParallelAxis::X => field_line.start.y - rotated_projection.y,
                    ParallelAxis::Y => field_line.start.x - rotated_projection.x,
                };

                let measurement = LineMeasurement {
                    distance: measured,
                    angle: measured_angle,
                };

                let update_covariance = {
                    let rotated_covariance = rotation
                        * CovarianceMatrix::from_diagonal_element(correspondence.error)
                        * rotation.transpose();

                    let distance_variance = match axis {
                        ParallelAxis::X => rotated_covariance[(1, 1)],
                        ParallelAxis::Y => rotated_covariance[(0, 0)],
                    };

                    let angle_variance = (4.0 * distance_variance
                        / (correspondence.detected_line.length().powi(2)))
                    .sqrt()
                    .atan()
                    .powi(2);

                    CovarianceMatrix::<2>::new(distance_variance, 0.0, 0.0, angle_variance)
                };

                particle
                    .update(
                        |pose| {
                            let state: StateVector<3> = pose.into();

                            match axis {
                                ParallelAxis::X => LineMeasurement {
                                    distance: state.y,
                                    angle: state.z,
                                },
                                ParallelAxis::Y => LineMeasurement {
                                    distance: state.x,
                                    angle: state.z,
                                },
                            }
                        },
                        measurement,
                        update_covariance,
                    )
                    .unwrap();

                particle.score += PARTICLE_SCORE_INCREASE;
                if correspondence.error.sqrt() < PARTICLE_BONUS_THRESHOLD {
                    particle.score += PARTICLE_SCORE_BONUS;
                }
            }
        }
    }
}

fn filter_particles(
    mut commands: Commands,
    mut pose: ResMut<RobotPose>,
    particles: Query<(Entity, &RobotPoseFilter)>,
) {
    let best_particle = particles
        .iter()
        .map(|x| x.1)
        .max_by(|a, b| a.score.total_cmp(&b.score))
        .expect("There should always be at least one particle.");

    for (entity, particle) in &particles {
        if particle.score < PARTICLE_RETAIN_FACTOR * best_particle.score {
            commands.entity(entity).despawn();
        }
    }

    *pose = best_particle.prediction();
}

fn resample(
    mut commands: Commands,
    layout: Res<LayoutConfig>,
    mut particles: Query<(Entity, &mut RobotPoseFilter)>,
) {
    // TODO: implement resampling
}

fn sensor_resetting(mut commands: Commands, mut particles: Query<(Entity, &mut RobotPoseFilter)>) {
    // TODO: implement sensor resetting based on field features from which the pose can directly be derived
}

fn log_particles(dbg: DebugContext, cycle: Res<Cycle>, particles: Query<&RobotPoseFilter>) {
    let particles = particles.iter().collect::<Vec<_>>();

    let best_particle_idx = particles
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.score.total_cmp(&b.score))
        .map(|(index, _)| index)
        .unwrap();

    dbg.log_with_cycle(
        "particles",
        *cycle,
        &rerun::Boxes3D::from_centers_and_half_sizes(
            (0..particles.len()).map(|_| (0.0, 0.0, 0.0)),
            (0..particles.len()).map(|_| (0.1, 0.1, 0.1)),
        )
        .with_colors((0..particles.len()).map(|i| {
            if i == best_particle_idx {
                (0, 255, 0)
            } else {
                (255, 0, 0)
            }
        })),
    );

    dbg.log_with_cycle(
        "particles",
        *cycle,
        &rerun::Transform3D::update_fields().with_axis_length(0.25),
    );

    dbg.log_with_cycle(
        "particles",
        *cycle,
        &rerun::InstancePoses3D::new()
            .with_translations(particles.iter().map(|particle| {
                (
                    particle.prediction().inner.translation.x,
                    particle.prediction().inner.translation.y,
                    0.0,
                )
            }))
            .with_rotation_axis_angles(particles.iter().map(|particle| {
                (
                    (0.0, 0.0, 1.0),
                    particle.prediction().inner.rotation.angle(),
                )
            })),
    );
}

#[derive(Debug, Clone, Copy)]
struct LineMeasurement {
    distance: f32,
    angle: f32,
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

fn initial_particles(
    layout: &LayoutConfig,
    player_num: u8,
) -> impl IntoIterator<Item = RobotPoseFilter> {
    let position = RobotPose {
        inner: layout.initial_positions.player(player_num).isometry,
    };

    (0..20).map(move |_| RobotPoseFilter {
        filter: UnscentedKalmanFilter::new(position, CovarianceMatrix::from_diagonal_element(0.1)),
        score: PARTICLE_SCORE_DEFAULT,
    })
}

/// normalizes an angle to be in the range \[-pi, pi\]
fn normalize_angle(mut angle: f32) -> f32 {
    use std::f32::consts::{PI, TAU};

    angle %= TAU;
    if angle > PI {
        angle -= TAU;
    } else if angle < -PI {
        angle += TAU;
    }
    angle
}
