use bevy::prelude::*;
use heimdall::{CameraLocation, CameraMatrix, Top};
use nalgebra::Point2;
use odal::Config;
use serde::{Deserialize, Serialize};

use crate::{
    core::debug::DebugContext,
    localization::RobotPose,
    nao::Cycle,
    prelude::ConfigExt,
    vision::{
        ball_detection::{ball_tracker::BallTracker, classifier::classify_balls},
        line_detection::{DetectedLines, detect_lines_system, line::LineSegment2},
        scan_lines::{RegionColor, ScanLines},
    },
};

#[derive(Resource, Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
struct NaiveRobotDetectionConfig {
    /// Minimum number of points in a cluster to be considered a cluster
    min_cluster_count: usize,

    /// Minimum distance of a point from a line to be considered for a cluster
    min_line_distance: f32,
    /// Minimum distance of a point from a ball to be considered for a cluster
    min_ball_distance: f32,

    /// Maximum distance between two points to be considered a cluster
    max_cluster_distance: f32,

    /// Minimum radius for a cluster to be used for robot detection
    min_cluster_radius: f32,
    /// Maximum radius for a cluster to be used for robot detection
    max_cluster_radius: f32,
}

impl Config for NaiveRobotDetectionConfig {
    const PATH: &'static str = "naive_robot_detection.toml";
}

/// Plugin for naive robot detection using the vertical scan lines.
pub struct NaiveRobotDetectionPlugin;

impl Plugin for NaiveRobotDetectionPlugin {
    fn build(&self, app: &mut App) {
        app.init_config::<NaiveRobotDetectionConfig>()
            .init_resource::<NaiveDetectedRobots>()
            .add_systems(PostStartup, setup_visualizations)
            .add_systems(
                Update,
                (
                    detect_robots
                        .after(classify_balls::<Top>)
                        .after(detect_lines_system::<Top>)
                        .run_if(resource_exists_and_changed::<ScanLines<Top>>),
                    visualize_detections,
                )
                    .chain(),
            );
    }
}

struct Cluster {
    /// Center of the cluster in field coordinates
    center: Point2<f32>,
    /// Width of the cluster
    width: f32,
    /// Number of points in the cluster
    count: usize,
}

impl Cluster {
    fn from_points(points: Vec<Point2<f32>>) -> Self {
        let center = points
            .iter()
            .fold(Point2::<f32>::origin(), |acc, point| acc + point.coords)
            / points.len() as f32;

        let width = points
            .iter()
            .map(|point| nalgebra::distance(&center, point))
            .fold(0.0, f32::max);

        let points = points.len();

        Self {
            center,
            width,
            count: points,
        }
    }
}

#[derive(Resource, Debug, Default, Clone)]
struct NaiveDetectedRobots {
    positions: Vec<Point2<f32>>,
    cycle: Cycle,
}

#[allow(clippy::too_many_arguments)]
fn detect_robots(
    dbg: DebugContext,
    robot_pose: Res<RobotPose>,
    scan_lines: Res<ScanLines<Top>>,
    camera_matrix: Res<CameraMatrix<Top>>,
    detected_lines: Query<&DetectedLines, Added<DetectedLines>>,
    ball_tracker: Res<BallTracker>,
    config: Res<NaiveRobotDetectionConfig>,
    mut detected_robots: ResMut<NaiveDetectedRobots>,
    mut segments: Local<Vec<LineSegment2>>,
) {
    if !detected_lines.is_empty() {
        *segments = detected_lines
            .iter()
            .flat_map(|line| line.segments.clone())
            .collect::<Vec<_>>();
    }

    // Get the cluster points from the scan lines.
    //
    // Points are ordered by the vertical scan lines and represent the lowest
    // point of the lowest white region in each vertical scan line
    let cluster_points = get_cluster_points(&scan_lines);

    // project the cluster points to the ground plane
    let mut projected_points = project_cluster_points(cluster_points, &camera_matrix);

    projected_points.retain(|point| {
        segments
            .iter()
            .all(|line| line.distance_to_point(*point) > config.min_line_distance)
            && ball_tracker
                .stationary_ball()
                .is_none_or(|ball| nalgebra::distance(&ball, point) > config.min_ball_distance)
    });

    dbg.log_with_cycle(
        Top::make_entity_path("naive_detected_robots/filtered_points"),
        detected_robots.cycle,
        &rerun::Points3D::new(projected_points.iter().map(|point| {
            let point = robot_pose.robot_to_world(point);
            (point.coords.x, point.coords.y, 0.1)
        }))
        .with_colors((0..projected_points.len()).map(|_| (255, 0, 0))),
    );

    // filter out points that are in lines or balls

    let mut clusters = create_clusters(projected_points, &config);

    clusters.retain(|cluster| {
        // filter out clusters that are too small or too large
        cluster.width > config.min_cluster_radius
            && cluster.width < config.max_cluster_radius
            && cluster.count > config.min_cluster_count
    });

    *detected_robots = NaiveDetectedRobots {
        positions: clusters.into_iter().map(|cluster| cluster.center).collect(),
        cycle: scan_lines.image().cycle(),
    };
}

fn setup_visualizations(dbg: DebugContext) {
    dbg.log_static(
        Top::make_entity_path("naive_detected_robots"),
        &rerun::Ellipsoids3D::update_fields().with_colors([(0, 0, 255)]),
    );
}

fn visualize_detections(
    dbg: DebugContext,
    detected_robots: Res<NaiveDetectedRobots>,
    robot_pose: Res<RobotPose>,
) {
    dbg.log_with_cycle(
        Top::make_entity_path("naive_detected_robots"),
        detected_robots.cycle,
        &rerun::Ellipsoids3D::from_centers_and_radii(
            detected_robots.positions.iter().map(|point| {
                let point = robot_pose.robot_to_world(point);
                (point.coords.x, point.coords.y, 0.1)
            }),
            (0..detected_robots.positions.len()).map(|_| 0.2),
        ),
    );
}

fn get_cluster_points(scan_lines: &ScanLines<Top>) -> Vec<Point2<f32>> {
    let mut cluster_points = vec![];

    let mut current_fixed_point: Option<usize> = None;
    let mut lowest_white_point_in_region: Option<Point2<f32>> = None;
    for region in scan_lines.vertical().regions() {
        // every vertical scan line we can get a new cluster point
        if current_fixed_point.is_none_or(|current| current != region.fixed_point()) {
            if let Some(lowest_white_point) = lowest_white_point_in_region {
                cluster_points.push(lowest_white_point);
            }

            current_fixed_point = Some(region.fixed_point());
            lowest_white_point_in_region = None;
        }

        let end_point = Point2::new(region.fixed_point() as f32, region.end_point() as f32);
        // we consider the lowest white region for the cluster point
        if matches!(region.color(), RegionColor::WhiteOrBlack)
            && lowest_white_point_in_region.is_none_or(|point| point.y < end_point.y)
        {
            lowest_white_point_in_region = Some(end_point);
        }
    }

    if let Some(point) = lowest_white_point_in_region {
        cluster_points.push(point);
    }

    cluster_points
}

fn project_cluster_points(
    cluster_points: Vec<Point2<f32>>,
    camera_matrix: &CameraMatrix<Top>,
) -> Vec<Point2<f32>> {
    cluster_points
        .into_iter()
        .filter_map(|point| camera_matrix.pixel_to_ground(point, 0.0).ok())
        .map(|point| point.xy())
        .collect::<Vec<_>>()
}

fn create_clusters(
    cluster_points: Vec<Point2<f32>>,
    config: &NaiveRobotDetectionConfig,
) -> Vec<Cluster> {
    let mut clusters = vec![];

    let mut current_cluster: Vec<Point2<f32>> = vec![];
    let mut last_point: Option<Point2<f32>> = None;
    for point in cluster_points {
        // if the current cluster is empty, we can start a new one
        let Some(_) = last_point else {
            current_cluster.push(point);
            last_point = Some(point);
            continue;
        };

        if current_cluster
            .iter()
            .any(|p| nalgebra::distance(&point, p) < config.max_cluster_distance)
        {
            current_cluster.push(point);
        } else {
            clusters.push(Cluster::from_points(std::mem::take(&mut current_cluster)));
            current_cluster.push(point);
        }
    }

    if !current_cluster.is_empty() {
        clusters.push(Cluster::from_points(current_cluster));
    }

    clusters
}
