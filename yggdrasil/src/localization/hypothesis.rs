use bevy::prelude::*;
use bifrost::communication::{GameControllerMessage, GamePhase, Penalty};
use filter::{
    CovarianceMatrix, StateMatrix, StateTransform, StateVector, UnscentedKalmanFilter, WeightVector,
};
use nalgebra::{point, vector, ComplexField, Point2, Rotation2, UnitComplex};
use num::Complex;

use crate::{
    core::config::{
        layout::{FieldLine, LayoutConfig, ParallelAxis},
        showtime::PlayerConfig,
    },
    game_controller::penalty::PenaltyState,
    motion::odometry::Odometry,
    vision::line_detection::DetectedLines,
};

use super::{
    correction::fit_field_lines,
    correspondence::FieldLineCorrespondence,
    pose::{penalized_pose, penalty_kick_pose},
    LocalizationConfig, RobotPose,
};

pub fn odometry_update(
    cfg: Res<LocalizationConfig>,
    odometry: Res<Odometry>,
    mut hypotheses: Query<&mut RobotPoseHypothesis>,
) {
    for mut hypothesis in &mut hypotheses {
        let _ = hypothesis
            .filter
            .predict(
                |pose| RobotPose {
                    inner: pose.inner * odometry.offset_to_last,
                },
                CovarianceMatrix::from_diagonal(&cfg.hypothesis.odometry_variance.into()),
            )
            .inspect_err(|_| warn!("Cholesky failed in odometry"));

        // TODO: why is this necessary?
        hypothesis.fix_covariance();

        hypothesis.score *= cfg.hypothesis.score_decay;
    }
}

pub fn line_update(
    cfg: Res<LocalizationConfig>,
    layout: Res<LayoutConfig>,
    new_lines: Query<&DetectedLines, Added<DetectedLines>>,
    mut hypotheses: Query<&mut RobotPoseHypothesis>,
) {
    // get the measured lines in robot space
    let segments = new_lines
        .iter()
        .flat_map(|lines| &lines.segments)
        .collect::<Vec<_>>();

    if segments.is_empty() {
        return;
    }

    for mut hypothesis in &mut hypotheses {
        let pose = hypothesis.filter.state();

        // get measured lines in field space
        let measured = segments
            .iter()
            .map(|&&segment| pose.inner * segment)
            .collect::<Vec<_>>();

        let Some((correspondences, fit_error)) = fit_field_lines(&measured, &cfg, &layout) else {
            continue;
        };

        let clamped_fit_error = fit_error.max(cfg.correspondence.min_fit_error);
        let num_measurements_weight = 1.0 / correspondences.len() as f32;

        for correspondence in correspondences {
            let line_length = correspondence.measurement.length();
            let line_length_weight = if line_length == 0.0 {
                1.0
            } else {
                1.0 / line_length
            };

            let line_center = correspondence.measurement.center();
            let line_distance_weight = nalgebra::distance(&line_center, &pose.world_position());

            let covariance_weight = clamped_fit_error
                * num_measurements_weight
                * line_length_weight
                * line_distance_weight;

            match correspondence.reference {
                FieldLine::Segment { axis, .. } => {
                    let _ = hypothesis
                        .filter
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
                            LineMeasurement::from_pose_and_correspondence(pose, &correspondence),
                            CovarianceMatrix::from_diagonal(
                                &cfg.hypothesis.line_measurement_variance.into(),
                            ) * covariance_weight,
                        )
                        .inspect_err(|_| warn!("Cholesky failed in line update"));
                }
                FieldLine::Circle(..) => {
                    let _ = hypothesis
                        .filter
                        .update(
                            |pose| {
                                let state: StateVector<3> = pose.into();
                                CircleMeasurement::from(state.xy())
                            },
                            CircleMeasurement::from_pose_and_correspondence(pose, &correspondence),
                            CovarianceMatrix::from_diagonal(
                                &cfg.hypothesis.circle_measurement_variance.into(),
                            ) * covariance_weight,
                        )
                        .inspect_err(|_| warn!("Cholesky failed in circle update"));
                }
            }

            if correspondence.error() < cfg.hypothesis.score_correspondence_bonus_threshold {
                hypothesis.score += cfg.hypothesis.score_correspondence_bonus;
            }
        }

        hypothesis.score += cfg.hypothesis.score_default_increase;
    }
}

pub fn filter_hypotheses(
    mut commands: Commands,
    mut pose: ResMut<RobotPose>,
    cfg: Res<LocalizationConfig>,
    hypotheses: Query<(Entity, &RobotPoseHypothesis)>,
) {
    let (new_pose, best_score) = hypotheses
        .iter()
        .max_by(|(_, a), (_, b)| a.score.total_cmp(&b.score))
        .map(|(_, hypothesis)| (hypothesis.filter.state(), hypothesis.score))
        .expect("Could not get best hypothesis");

    // remove all hypotheses that are not good enough
    for (entity, hypothesis) in hypotheses.iter() {
        if hypothesis.score < cfg.hypothesis.retain_ratio * best_score {
            commands.entity(entity).despawn();
        }
    }

    // set the new best pose
    *pose = new_pose;
}

/// Checks if the penalty should be in place (and not be placed on the side of the field)
fn is_penalized_in_place(penalty: Penalty) -> bool {
    matches!(
        penalty,
        Penalty::IllegalMotionInStandby | Penalty::IllegalMotionInSet
    )
}

/// Resets the hypotheses to a known state based on game conditions
pub fn reset_hypotheses(
    mut commands: Commands,
    mut hypotheses: Query<Entity, With<RobotPoseHypothesis>>,
    penalty_state: Res<PenaltyState>,
    layout: Res<LayoutConfig>,
    player: Res<PlayerConfig>,
    localization: Res<LocalizationConfig>,
    gcm: Option<Res<GameControllerMessage>>,
) {
    if penalty_state.entered_penalty() && !is_penalized_in_place(penalty_state.current()) {
        for entity in &mut hypotheses {
            commands.entity(entity).despawn();
        }

        for pose in penalized_pose(&layout) {
            commands.spawn(RobotPoseHypothesis::new(
                pose,
                CovarianceMatrix::from_diagonal(&localization.hypothesis.variance_initial.into()),
                localization.hypothesis.score_initial,
            ));
        }
    }

    if let Some(gcm) = gcm {
        if matches!(gcm.game_phase, GamePhase::PenaltyShoot) {
            for entity in &mut hypotheses {
                commands.entity(entity).despawn();
            }

            let is_kicking_team = gcm.kicking_team == player.team_number;
            let pose = penalty_kick_pose(&layout, is_kicking_team);

            commands.spawn(RobotPoseHypothesis::new(
                pose,
                CovarianceMatrix::from_diagonal(&localization.hypothesis.variance_initial.into()),
                localization.hypothesis.score_initial,
            ));
        }
    }
}

#[derive(Clone, Component)]
pub struct RobotPoseHypothesis {
    filter: UnscentedKalmanFilter<3, 7, RobotPose>,
    pub score: f32,
}

impl RobotPoseHypothesis {
    #[must_use]
    pub fn new(
        initial_pose: RobotPose,
        initial_covariance: CovarianceMatrix<3>,
        initial_score: f32,
    ) -> Self {
        let filter =
            UnscentedKalmanFilter::<3, 7, RobotPose>::new(initial_pose, initial_covariance);

        Self {
            filter,
            score: initial_score,
        }
    }

    /// Mitigate numerical instability with covariance matrix by ensuring it is symmetric
    pub fn fix_covariance(&mut self) {
        let cov = &mut self.filter.covariance;
        *cov = (*cov + cov.transpose()) * 0.5;
    }
}

#[derive(Debug, Clone, Copy)]
struct LineMeasurement {
    distance: f32,
    angle: f32,
}

impl LineMeasurement {
    fn from_pose_and_correspondence(
        pose: RobotPose,
        correspondence: &FieldLineCorrespondence,
    ) -> Self {
        let FieldLine::Segment { segment, axis } = correspondence.reference else {
            panic!("Tried to make line measurements from circle measurements");
        };

        let measurement_needs_flipping = match axis {
            ParallelAxis::X => {
                correspondence.measurement.end.x < correspondence.measurement.start.x
            }
            ParallelAxis::Y => {
                correspondence.measurement.end.y < correspondence.measurement.start.y
            }
        };

        let measurement = if measurement_needs_flipping {
            correspondence.measurement.to_flipped()
        } else {
            correspondence.measurement
        };

        let measurement_vector = measurement.end - measurement.start;
        let signed_distance = measurement
            .to_line()
            .signed_distance_to_point(pose.world_position());

        match axis {
            ParallelAxis::X => LineMeasurement {
                distance: segment.start.y + signed_distance,
                angle: (-measurement_vector.y).atan2(measurement_vector.x) + pose.world_rotation(),
            },
            ParallelAxis::Y => LineMeasurement {
                distance: segment.start.x - signed_distance,
                angle: measurement_vector.x.atan2(measurement_vector.y) + pose.world_rotation(),
            },
        }
    }
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
        weights: WeightVector<N>,
        states: StateMatrix<2, N>,
    ) -> StateVector<2> {
        let mut mean_distance = 0.0;
        let mut mean_angle = Complex::ZERO;

        for (&weight, pose) in weights.iter().zip(states.column_iter()) {
            mean_distance += weight * pose.x;
            mean_angle += weight * Complex::cis(pose.y);
        }

        vector![mean_distance, mean_angle.argument()]
    }

    fn residual(measurement: StateVector<2>, prediction: StateVector<2>) -> StateVector<2> {
        vector![
            measurement.x - prediction.x,
            (UnitComplex::new(measurement.y) / UnitComplex::new(prediction.y)).angle()
        ]
    }
}

struct CircleMeasurement {
    position: Point2<f32>,
}

impl CircleMeasurement {
    fn from_pose_and_correspondence(
        pose: RobotPose,
        correspondence: &FieldLineCorrespondence,
    ) -> Self {
        let measurement_vector = correspondence.end.measurement - correspondence.start.measurement;
        let reference_vector = correspondence.end.reference - correspondence.start.reference;

        let measurement_start_to_robot = pose.world_position() - correspondence.start.measurement;

        // Signed angle between two vectors: https://wumbo.net/formulas/angle-between-two-vectors-2d/
        let measured_rotation = f32::atan2(
            measurement_start_to_robot.y * measurement_vector.x
                - measurement_start_to_robot.x * measurement_vector.y,
            measurement_start_to_robot.x * measurement_vector.x
                + measurement_start_to_robot.y * measurement_vector.y,
        );

        let reference_start_to_robot = Rotation2::new(measured_rotation)
            * reference_vector.normalize()
            * measurement_start_to_robot.norm();

        let position = correspondence.start.reference + reference_start_to_robot;

        CircleMeasurement { position }
    }
}

impl From<StateVector<2>> for CircleMeasurement {
    fn from(value: StateVector<2>) -> Self {
        CircleMeasurement {
            position: point![value.x, value.y],
        }
    }
}

impl From<CircleMeasurement> for StateVector<2> {
    fn from(value: CircleMeasurement) -> Self {
        vector![value.position.x, value.position.y]
    }
}

impl StateTransform<2> for CircleMeasurement {
    // only uses linear values (no angles), so we can use the default impl
}
