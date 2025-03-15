use std::{
    iter::IntoIterator,
    time::{Duration, Instant},
};

use bevy::prelude::*;

use bifrost::communication::GameControllerMessage;
use filter::{
    CovarianceMatrix, StateMatrix, StateTransform, StateVector, UnscentedKalmanFilter, WeightVector,
};
use heimdall::{Bottom, Top};
use nalgebra::{self as na, point, vector, ComplexField, Point2, Rotation2};
use nidhogg::types::HeadJoints;

use crate::{
    behavior::primary_state::{is_penalized_by_game_controller, PrimaryState},
    core::{
        config::{
            layout::{FieldLine, LayoutConfig, ParallelAxis},
            showtime::PlayerConfig,
        },
        debug::DebugContext,
    },
    localization::correspondence::LineCorrespondences,
    motion::{keyframe::KeyframeExecutor, odometry::Odometry},
    nao::Cycle,
    sensor::fsr::Contacts,
};

use super::correspondence::get_correspondences;

const PARTICLE_SCORE_DECAY: f32 = 0.95;
const PARTICLE_SCORE_DEFAULT: f32 = 10.0;
const PARTICLE_SCORE_INCREASE: f32 = 0.5;
const PARTICLE_SCORE_BONUS: f32 = 2.5;
const PARTICLE_BONUS_THRESHOLD: f32 = 0.5;
const PARTICLE_RETAIN_FACTOR: f32 = 0.5;

pub struct PoseFilterPlugin;

impl Plugin for PoseFilterPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PrimaryStateHistory>()
            .init_resource::<PenalizedHistory>()
            .add_systems(PostStartup, initialize_particles_and_pose)
            .add_systems(
                Update,
                (
                    odometry_update
                        .run_if(not(motion_is_unsafe))
                        .run_if(not(is_penalized)),
                    update_primary_state_history,
                    update_penalized_history,
                    penalized_resetting,
                    line_update
                        // .run_if(returned_shortly_ago)
                        .run_if(not(motion_is_unsafe))
                        .run_if(not(is_penalized)),
                    filter_particles,
                    log_particles,
                )
                    .chain()
                    .after(get_correspondences::<Top>)
                    .after(get_correspondences::<Bottom>),
            );
    }
}

#[derive(Resource, Default, Debug, Clone, Copy)]
pub struct RobotPose {
    pub inner: na::Isometry2<f32>,
}

impl RobotPose {
    // Constant for camera height that we set anywhere get_lookat_absolute is called.
    // Set to zero if we are only looking at the ground, for example.
    pub const CAMERA_HEIGHT: f32 = 0.5;

    #[must_use]
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

    /// The current pose of the robot in the world, in 3D space.
    ///
    /// The z-axis is always 0.
    /// The rotation is around the z-axis.
    #[must_use]
    pub fn as_3d(&self) -> na::Isometry3<f32> {
        na::Isometry3::from_parts(
            na::Translation3::new(self.inner.translation.x, self.inner.translation.y, 0.0),
            na::UnitQuaternion::from_euler_angles(0.0, 0.0, self.inner.rotation.angle()),
        )
    }

    /// The current position of the robot in the world, in absolute coordinates.
    ///
    /// The center of the world is at the center of the field, with the x-axis pointing towards the
    /// opponent's goal.
    #[must_use]
    pub fn world_position(&self) -> na::Point2<f32> {
        self.inner.translation.vector.into()
    }

    /// The current rotation of the robot in the world, in radians.
    #[must_use]
    pub fn world_rotation(&self) -> f32 {
        self.inner.rotation.angle()
    }

    /// Transform a point from robot coordinates to world coordinates.
    #[must_use]
    pub fn robot_to_world(&self, point: &na::Point2<f32>) -> na::Point2<f32> {
        self.inner.transform_point(point)
    }

    /// Transform a point from world coordinates to robot coordinates.
    #[must_use]
    pub fn world_to_robot(&self, point: &na::Point2<f32>) -> na::Point2<f32> {
        self.inner.inverse_transform_point(point)
    }

    #[must_use]
    pub fn get_look_at_absolute(&self, point_in_world: &na::Point3<f32>) -> HeadJoints<f32> {
        let robot_to_point = self.world_to_robot(&point_in_world.xy());
        let x = robot_to_point.x;
        let y = robot_to_point.y;
        let z = point_in_world.z;
        let yaw = (robot_to_point.y / robot_to_point.x).atan();
        // 0.5 is the height of the robot's primary camera while standing
        let pitch = (0.5 - z).atan2((x * x + y * y).sqrt());

        HeadJoints { yaw, pitch }
    }

    #[must_use]
    pub fn distance_to(&self, point: &na::Point2<f32>) -> f32 {
        (self.world_position() - point).norm()
    }

    #[must_use]
    pub fn angle_to(&self, point: &na::Point2<f32>) -> f32 {
        let robot_to_point = self.world_to_robot(point).xy();
        robot_to_point.y.atan2(robot_to_point.x)
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
    ));
}

fn odometry_update(odometry: Res<Odometry>, mut particles: Query<&mut RobotPoseFilter>) {
    for mut particle in &mut particles {
        if particle
            .predict(
                |pose| RobotPose {
                    inner: pose.inner * odometry.offset_to_last,
                },
                CovarianceMatrix::from_diagonal(&na::Vector3::new(0.05, 0.05, 0.01)),
            )
            .is_err()
        {
            warn!("Cholesky failed in odometry");
            particle.covariance = CovarianceMatrix::from_diagonal(&na::Vector3::new(
                1.0,
                1.0,
                std::f32::consts::FRAC_PI_4,
            ));
        }

        particle.score *= PARTICLE_SCORE_DECAY;
    }
}

fn returned_shortly_ago(penalized_history: Res<PenalizedHistory>) -> bool {
    penalized_history.duration_since_return() < Duration::from_secs(8)
}

fn motion_is_unsafe(
    keyframe_executor: Res<KeyframeExecutor>,
    // motion_state: Res<State<Gait>>,
    contacts: Res<Contacts>,
) -> bool {
    keyframe_executor.active_motion.is_some()
        // || !matches!(motion_state.get(), Gait::Standing | Gait::Walking)
        || !contacts.ground
}

fn update_primary_state_history(
    mut primary_state_history: ResMut<PrimaryStateHistory>,
    primary_state: Res<PrimaryState>,
) {
    primary_state_history.previous = primary_state_history.current;
    primary_state_history.current = *primary_state;
}

fn is_penalized(gcm: Option<Res<GameControllerMessage>>, player_config: Res<PlayerConfig>) -> bool {
    is_penalized_by_game_controller(
        gcm.as_deref(),
        player_config.team_number,
        player_config.player_number,
    )
}

fn update_penalized_history(
    mut penalized_history: ResMut<PenalizedHistory>,
    gcm: Option<Res<GameControllerMessage>>,
    player_config: Res<PlayerConfig>,
) {
    penalized_history.previous = penalized_history.current;
    penalized_history.current = is_penalized_by_game_controller(
        gcm.as_deref(),
        player_config.team_number,
        player_config.player_number,
    );

    if !penalized_history.current && penalized_history.previous {
        penalized_history.last_return = Some(Instant::now());
    }
}

#[derive(Resource, Default, Debug, Clone, Copy)]
pub struct PenalizedHistory {
    previous: bool,
    current: bool,
    last_return: Option<Instant>,
}

impl PenalizedHistory {
    pub fn duration_since_return(&self) -> Duration {
        self.last_return.map_or(Duration::MAX, |last_return| {
            Instant::now().duration_since(last_return)
        })
    }
}

#[derive(Resource, Debug, Default, Clone, Copy)]
struct PrimaryStateHistory {
    previous: PrimaryState,
    current: PrimaryState,
}

fn penalized_resetting(
    mut commands: Commands,
    penalized_history: Res<PenalizedHistory>,
    particles: Query<Entity, With<RobotPoseFilter>>,
    layout: Res<LayoutConfig>,
    robot_pose: Res<RobotPose>,
) {
    match (penalized_history.previous, penalized_history.current) {
        (false, true) => {
            for entity in &particles {
                commands.entity(entity).despawn();
            }

            for particle in penalized_particles(&layout, *robot_pose) {
                commands.spawn(particle);
            }
        }
        (_, _) => (),
    };

    // TODO: penalty shootout
}

fn line_update(
    layout: Res<LayoutConfig>,
    added_correspondences: Query<&LineCorrespondences, Added<LineCorrespondences>>,
    mut particles: Query<&mut RobotPoseFilter>,
) {
    for mut particle in &mut particles {
        for correspondences in &added_correspondences {
            for correspondence in correspondences.iter() {
                match correspondence.field_line {
                    FieldLine::Segment {
                        segment: field_line,
                        axis,
                    } => {
                        if correspondence
                            .detected_line
                            .normal()
                            .angle(&field_line.normal())
                            > std::f32::consts::FRAC_PI_6
                        {
                            continue;
                        }

                        let current_pose = particle.state();

                        if !layout.field.in_field_with_margin(field_line.start, 0.15)
                            || !layout.field.in_field_with_margin(field_line.end, 0.15)
                        {
                            continue;
                        }

                        // line from the robot to the detected line
                        let relative_line = (correspondence.pose.inner.inverse()
                            * correspondence.detected_line)
                            .to_line();

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
                            if normalize_angle(measured_angle_alternative - current_pose.angle())
                                .abs()
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

                            let line_length_weight = if correspondence.detected_line.length() == 0.0
                            {
                                1.0
                            } else {
                                1.0 / correspondence.detected_line.length()
                            };

                            let angle_variance = (4.0 * distance_variance
                                / (correspondence.detected_line.length().powi(2)))
                            .sqrt()
                            .atan()
                            .powi(2);

                            CovarianceMatrix::<2>::new(
                                line_length_weight * distance_variance,
                                0.0,
                                0.0,
                                angle_variance,
                            )
                        };

                        particle.fix_covariance();

                        if particle
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
                            .is_err()
                        {
                            continue;
                        }
                    }
                    FieldLine::Circle(..) => {
                        let current_pose = particle.state();

                        if !layout
                            .field
                            .in_field_with_margin(current_pose.position(), 0.15)
                        {
                            continue;
                        }

                        // line from the robot to the detected line
                        let measured =
                            correspondence.pose.inner.inverse() * correspondence.detected_line;
                        let reference =
                            correspondence.pose.inner.inverse() * correspondence.projected_line;

                        if measured.normal().angle(&reference.normal())
                            > std::f32::consts::FRAC_PI_8
                        {
                            continue;
                        }

                        let measured_line_vector = measured.end - measured.start;
                        let reference_line_vector = reference.end - reference.start;
                        let measured_line_point_start_to_robot_vector =
                            correspondence.pose.position() - measured.start;

                        // Signed angle between two vectors: https://wumbo.net/formulas/angle-between-two-vectors-2d/
                        let measured_rotation = f32::atan2(
                            measured_line_point_start_to_robot_vector.y * measured_line_vector.x
                                - measured_line_point_start_to_robot_vector.x
                                    * measured_line_vector.y,
                            measured_line_point_start_to_robot_vector.x * measured_line_vector.x
                                + measured_line_point_start_to_robot_vector.y
                                    * measured_line_vector.y,
                        );

                        let reference_line_point_start_to_robot_vector =
                            Rotation2::new(measured_rotation)
                                * reference_line_vector.normalize()
                                * measured_line_point_start_to_robot_vector.norm();

                        let reference_robot_point =
                            reference.start + reference_line_point_start_to_robot_vector;

                        let measurement = CircleMeasurement {
                            position: reference_robot_point,
                        };

                        let line_length_weight = if correspondence.detected_line.length() == 0.0 {
                            1.0
                        } else {
                            1.0 / correspondence.detected_line.length()
                        };

                        if particle
                            .update(
                                |pose| {
                                    let state: StateVector<3> = pose.into();
                                    CircleMeasurement::from(state.xy())
                                },
                                measurement,
                                CovarianceMatrix::from_diagonal_element(
                                    correspondence.error * line_length_weight,
                                ),
                            )
                            .is_err()
                        {
                            warn!("cholesky failed");
                            particle.covariance = CovarianceMatrix::from_diagonal(
                                &na::Vector3::new(1.0, 1.0, std::f32::consts::FRAC_PI_4),
                            );
                            continue;
                        };
                    }
                }

                particle.score += PARTICLE_SCORE_INCREASE;
                if correspondence.error.sqrt() < PARTICLE_BONUS_THRESHOLD {
                    particle.score += PARTICLE_SCORE_BONUS;
                } else {
                    particle.score *= 0.9;
                }
            }
        }
    }
    for mut particle in &mut particles {
        particle.fix_covariance();
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

    // for (entity, particle) in &particles {
    //     if particle.score < PARTICLE_RETAIN_FACTOR * best_particle.score {
    //         commands.entity(entity).despawn();
    //     }
    // }

    *pose = best_particle.state();

    let particle_count = particles.iter().count();
}

// TODO: implement particle resampling
fn resample(
    mut _commands: Commands,
    _layout: Res<LayoutConfig>,
    mut _particles: Query<(Entity, &mut RobotPoseFilter)>,
) {
}

fn sensor_resetting(
    mut _commands: Commands,
    mut _particles: Query<(Entity, &mut RobotPoseFilter)>,
) {
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
                    particle.state().inner.translation.x,
                    particle.state().inner.translation.y,
                    0.0,
                )
            }))
            .with_rotation_axis_angles(
                particles
                    .iter()
                    .map(|particle| ((0.0, 0.0, 1.0), particle.state().inner.rotation.angle())),
            ),
    );

    dbg.log_with_cycle(
        "particles",
        *cycle,
        &rerun::Points3D::new(particles.iter().map(|particle| {
            (
                particle.state().inner.translation.x,
                particle.state().inner.translation.y,
                0.0,
            )
        }))
        .with_labels(
            particles
                .iter()
                .map(|particle| format!("{:.2}", particle.score)),
        ),
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

struct CircleMeasurement {
    position: Point2<f32>,
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

impl StateTransform<2> for CircleMeasurement {}

fn noisy_isometry(x_max: f32, y_max: f32, angle_max: f32) -> na::Isometry2<f32> {
    let translation = na::Translation2::new(
        rand::random::<f32>() * x_max * 2.0 - x_max,
        rand::random::<f32>() * y_max * 2.0 - y_max,
    );
    let angle = Rotation2::new(rand::random::<f32>() * angle_max * 2.0 - angle_max);
    na::Isometry2::from_parts(translation, angle.into())
}

fn initial_particles(
    layout: &LayoutConfig,
    player_num: u8,
) -> impl IntoIterator<Item = RobotPoseFilter> {
    let position = RobotPose {
        inner: layout.initial_positions.player(player_num).isometry,
    };

    std::iter::once(RobotPoseFilter {
        filter: UnscentedKalmanFilter::new(position, CovarianceMatrix::from_diagonal_element(0.1)),
        score: PARTICLE_SCORE_DEFAULT,
    })
    .chain((0..10).map(move |_| {
        let noise_isom = noisy_isometry(0.1, 0.1, 0.05);
        RobotPoseFilter {
            filter: UnscentedKalmanFilter::new(
                RobotPose {
                    inner: position.inner * noise_isom,
                },
                CovarianceMatrix::from_diagonal_element(0.1),
            ),
            score: PARTICLE_SCORE_DEFAULT,
        }
    }))
}

fn penalized_particles(
    layout: &LayoutConfig,
    last_known_position: RobotPose,
) -> impl IntoIterator<Item = RobotPoseFilter> + use<'_> {
    const PARTICLES_PER_SIDE: u32 = 10;

    // negative sign, we are on our side of the field
    // positive sign, we are on the opponents side
    let side_sign = last_known_position.position().x.signum();

    (0..PARTICLES_PER_SIDE)
        .map(move |i| {
            let frac = i as f32 / PARTICLES_PER_SIDE as f32;

            let angle = Rotation2::new(-std::f32::consts::FRAC_PI_2);
            let pos = na::Translation2::new(
                side_sign * frac * layout.field.length * 0.5,
                layout.field.width * 0.5,
            );
            let pose = RobotPose {
                inner: na::Isometry2::from_parts(pos, angle.into()),
            };

            RobotPoseFilter {
                filter: UnscentedKalmanFilter::new(
                    pose,
                    CovarianceMatrix::from_diagonal_element(0.5),
                ),
                score: PARTICLE_SCORE_DEFAULT,
            }
        })
        .chain((0..PARTICLES_PER_SIDE).map(move |i| {
            let frac = i as f32 / PARTICLES_PER_SIDE as f32;

            let angle = Rotation2::new(std::f32::consts::FRAC_PI_2);
            let pos = na::Translation2::new(
                side_sign * frac * layout.field.length * 0.5,
                -layout.field.width * 0.5,
            );
            let pose = RobotPose {
                inner: na::Isometry2::from_parts(pos, angle.into()),
            };

            RobotPoseFilter {
                filter: UnscentedKalmanFilter::new(
                    pose,
                    CovarianceMatrix::from_diagonal_element(0.5),
                ),
                score: PARTICLE_SCORE_DEFAULT,
            }
        }))

    // we are on the opponents side
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
