use std::iter::IntoIterator;

use bevy::prelude::*;

use filter::{
    CovarianceMatrix, StateMatrix, StateTransform, StateVector, UnscentedKalmanFilter, WeightVector,
};
use nalgebra::{self as na, point, vector, ComplexField};

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
    vision::line_detection::line::LineSegment2,
};

use super::correspondence::{self, LineCorrespondences};

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

#[derive(Component, Deref, DerefMut)]
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
    correspondences: Query<&LineCorrespondences, Added<LineCorrespondences>>,
    mut particles: Query<&mut RobotPoseFilter>,
) {
    for mut particle in &mut particles {
        for new_correspondences in correspondences.iter() {
            for correspondence in &new_correspondences.0 {
                // skip circles for now
                if matches!(correspondence.field_line, FieldLine::Circle(_)) {
                    continue;
                }

                let Some(direction) = correspondence.field_line.direction() else {
                    continue;
                };

                // the line that the robot actually saw
                let detected = match direction {
                    Direction::AlongX
                        if correspondence.detected_line.end.x
                            < correspondence.detected_line.start.x =>
                    {
                        LineSegment2::new(
                            correspondence.detected_line.end,
                            correspondence.detected_line.start,
                        )
                    }
                    Direction::AlongY
                        if correspondence.detected_line.end.y
                            < correspondence.detected_line.start.y =>
                    {
                        LineSegment2::new(
                            correspondence.detected_line.end,
                            correspondence.detected_line.start,
                        )
                    }
                    _ => correspondence.detected_line,
                };

                // the line on the field that the robot is expected to see
                let projected = correspondence.projected_line;

                // TODO: remove and make validity check
                if projected.length() < 1.0 {
                    continue;
                }

                let distance = match direction {
                    Direction::AlongX => detected.center().y - projected.center().y,
                    Direction::AlongY => detected.center().x - projected.center().x,
                };

                // angle difference between the detected and projected lines
                let angle_diff = detected.normal().angle(&projected.normal());

                let measurement = LineMeasurement {
                    distance,
                    angle: angle_diff,
                };

                particle.fix_covariance();

                println!(
                    "Updating particle in direction {:?} with {:?}",
                    direction, measurement
                );

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
                        CovarianceMatrix::from_diagonal_element(1.0),
                    )
                    .unwrap();
            }
        }
    }
}

fn log_single(dbg: DebugContext, cycle: Res<Cycle>, particles: Query<&RobotPoseFilter>) {
    if let Some(a) = particles.iter().next() {
        let pos = a.state();

        dbg.log_with_cycle(
            "localization",
            *cycle,
            &rerun::Points3D::new([(pos.inner.translation.x, pos.inner.translation.y, 0.0)])
                .with_colors([(255, 0, 0)])
                .with_radii([0.2]),
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
