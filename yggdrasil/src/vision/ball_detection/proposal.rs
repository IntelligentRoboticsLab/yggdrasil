//! See [`BallProposalPlugin`].

use bevy::prelude::*;

use heimdall::{CameraLocation, CameraMatrix, CameraPosition};
use itertools::Itertools;
use nalgebra::{Point2, point};

use serde::{Deserialize, Serialize};

use crate::{
    core::debug::DebugContext,
    nao::Cycle,
    vision::{
        body_contour::{BodyContour, update_body_contours},
        camera::{Image, init_camera},
        scan_lines::{ClassifiedScanLineRegion, RegionColor, ScanLine, ScanLines},
        util::bbox::{Bbox, Xyxy},
    },
};

#[derive(Resource, Debug, Clone, Serialize, Deserialize)]
pub struct BallProposalConfigs {
    /// Top camera ball proposal configuration
    pub top: BallProposalConfig,
    /// Bottom camera ball proposal configuration
    pub bottom: BallProposalConfig,
}

/// Configurable values for getting ball proposals during the ball detection pipeline
#[derive(Debug, Clone, Resource, Serialize, Deserialize)]
pub struct BallProposalConfig {
    /// The minimum ratio of white or black pixels in the range around the proposed ball
    pub ball_ratio: f32,
    /// The ratio of green pixels in the patch below the ball.
    pub grass_ratio: f32,
    /// Height/width of the bounding box around the ball
    pub bounding_box_scale: f32,
    /// The minimum overlap ratio between for bounding boxes to be merged using non-maximum suppression
    pub nms_threshold: f32,
    /// The minimum radius of the proposed ball in meters.
    pub min_ball_radius: f32,
    /// The maximum radius of the proposed ball in meters.
    pub max_ball_radius: f32,
    /// The maximum area of the intersection between a detected robot and a proposed ball in pixels.
    pub max_robot_intersection: f32,
    // Maximum allowed error factor on the computed ball radius.
    pub ball_radius_max_error: f32,
    // Y offset from the ball center from where the patch below the ball is checked for grass, as a factor of the ball radius.
    pub grass_patch_y_offset_factor: f32,
    // Radius of the patch below the ball that is checked for grass, as a factor of the ball radius.
    pub grass_patch_radius_factor: f32,
    /// Padding factor applied to final proposal box sizes to account for camera calibration error.
    pub bbox_padding_factor: f32,
}

/// Plugin for finding possible ball locations in the camera images.
pub struct BallProposalPlugin<T: CameraLocation>(std::marker::PhantomData<T>);

impl<T: CameraLocation> Default for BallProposalPlugin<T> {
    fn default() -> Self {
        Self(std::marker::PhantomData)
    }
}

impl<T: CameraLocation> Plugin for BallProposalPlugin<T> {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, init_ball_proposals::<T>.after(init_camera::<T>))
            .add_systems(
                Update,
                (
                    update_ball_proposals::<T>.after(update_body_contours),
                    log_ball_proposals::<T>,
                )
                    .chain()
                    .run_if(resource_exists_and_changed::<ScanLines<T>>),
            );
    }
}

/// Points at which a ball may possibly be located
#[derive(Clone, Resource)]
pub struct BallProposals<T: CameraLocation> {
    pub image: Image<T>,
    pub proposals: Vec<BallProposal>,
}

impl<T: CameraLocation> BallProposals<T> {
    fn empty(image: Image<T>) -> Self {
        Self {
            image,
            proposals: Vec::new(),
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct BallProposal {
    pub position: Point2<usize>,
    pub scale: f32,
    pub distance_to_ball: f32,
    pub bbox: Bbox<Xyxy>,
}

pub fn update_ball_proposals<T: CameraLocation>(
    mut ball_proposals: ResMut<BallProposals<T>>,
    scan_lines: Res<ScanLines<T>>,
    matrix: Res<CameraMatrix<T>>,
    configs: Res<BallProposalConfigs>,
    body_contour: Res<BodyContour>,
) {
    let config = match T::POSITION {
        CameraPosition::Top => &configs.top,
        CameraPosition::Bottom => &configs.bottom,
    };
    *ball_proposals = get_ball_proposals(&scan_lines, &matrix, config, &body_contour);

    // TODO: Add this back
    // if let Some(robots) = robots {
    //     // remove proposals that are too close to robots
    //     let robot_bboxes = robots.detected.iter().map(|robot| robot.bbox).collect_vec();

    //     new.proposals.retain(|proposal| {
    //         robot_bboxes.iter().all(|bbox| {
    //             let proposal_bbox = Bbox::cxcywh(
    //                 proposal.position.x as f32,
    //                 proposal.position.y as f32,
    //                 proposal.scale,
    //                 proposal.scale,
    //             );

    //             bbox.intersection(&proposal_bbox) <= config.max_robot_intersection
    //         })
    //     });
    // }
}

/// Computes the overlap of two lines.
#[must_use]
pub fn range_overlap((start0, end0): (f32, f32), (start1, end1): (f32, f32)) -> f32 {
    (end0.min(end1) - start0.max(start1)).max(0.0)
}

/// Computes the overlap of a scanline with the shape of the ball.
#[must_use]
pub fn compute_ball_overlap(
    ball_center: Point2<f32>,
    radius: f32,
    line: &ClassifiedScanLineRegion,
) -> f32 {
    // line y relative to ball center
    let y = line.fixed_point() as f32 - ball_center.y;
    // y lies below or above ball
    if y.abs() >= radius {
        return 0.0;
    }

    // circle boundary x coordinate relative to circle center
    let x = (radius * radius - y * y).sqrt();
    // overlap between shape and scanline
    range_overlap(
        (ball_center.x - x, ball_center.x + x),
        (line.start_point() as f32, line.end_point() as f32),
    )
}

/// Computes the overlap of a scanline with the shape of the area underneath the ball.
/// Specifically, that shape is the bottom half of the outer circle that encompasses the ball.
///
/// # Args
/// - `radius`: radius of the ball
/// - `start_y`: at which y the circle is cut off to determine the bottom part of the circle
/// - `outer_radius`: radius of the circle around the ball
#[must_use]
pub fn compute_surface_overlap(
    ball_center: Point2<f32>,
    radius: f32,
    start_y: f32,
    outer_radius: f32,
    line: &ClassifiedScanLineRegion,
) -> f32 {
    // line y relative to ball center
    let y = line.fixed_point() as f32 - ball_center.y;
    // ? and y lies on or outside outer circle
    if y < start_y || y >= outer_radius {
        return 0.0;
    }

    let inner_x = f32::max(0.0, (radius * radius - y * y).sqrt());
    let outer_x = (outer_radius * outer_radius - y * y).sqrt();

    // compute the overlap of the outer circle minus the ball circle
    let left_overlap = range_overlap(
        (ball_center.x - outer_radius, ball_center.x - inner_x),
        (line.start_point() as f32, line.end_point() as f32),
    );

    let right_overlap = range_overlap(
        (ball_center.x + inner_x, ball_center.x + outer_x),
        (line.start_point() as f32, line.end_point() as f32),
    );

    left_overlap + right_overlap
}

/// Checks if a proposed circular area in the camera image at center `ball_center` with radius `radius` is likely to
/// be a ball. If so, it is added to `proposals` so a more accurate, but more costly, classifier can later on
/// classify the sample.
///
/// The check consists out of two parts:
///
/// 1. Shape check: What are the colors that constitute the proposed circular area? If the colors are predominantly black and white, it could be a ball.
///
/// 2. What are the colors underneath the ball? We know the ball lies on grass, hence if the colors are predominantly green, the proposal could be a ball.
///
/// # Args
///
/// - `ball_center`: proposed ball center
/// - `radius`: expected ball radius for `ball_center`
/// - `distance`: distance from ball to camera
/// - `(image_width, image_height)`: dimension of the image.
#[must_use]
pub fn check_proposal(
    ball_center: Point2<f32>,
    radius: f32,
    distance: f32,
    h_lines: &ScanLine,
    config: &BallProposalConfig,
    (image_width, image_height): (f32, f32),
) -> Option<(BallProposal, (Bbox<Xyxy>, f32))> {
    // what parts of the the ball shape are colored white/black and another color
    let mut ball_overlap = (0.0, 0.0);
    // what parts of the area beneath the ball shape are colored green and another color
    let mut surface_overlap = (0.0, 0.0);

    // radius of concentric circle where we look for grass
    let outer_radius = radius * config.grass_patch_radius_factor;

    // for each scanline, compute how it overlaps with the shapes
    for line in h_lines.regions() {
        // skip iteration if current scan line is guaranteed to not overlap the area of interest
        if ball_center.y - line.fixed_point() as f32 > radius
            || line.start_point() as f32 - ball_center.x > outer_radius
            || ball_center.x - line.end_point() as f32 > outer_radius
        {
            continue;
        }
        // early exit if we are already past the area of interest
        if line.fixed_point() as f32 - ball_center.y > outer_radius {
            break;
        }

        let line_ball_overlap = compute_ball_overlap(ball_center, radius, line);
        let line_surface_overlap = compute_surface_overlap(
            ball_center,
            radius,
            radius * config.grass_patch_y_offset_factor,
            outer_radius,
            line,
        );

        match line.color() {
            RegionColor::WhiteOrBlack => {
                ball_overlap.0 += line_ball_overlap;
                surface_overlap.1 += line_surface_overlap;
            }
            RegionColor::Green => {
                ball_overlap.1 += line_ball_overlap;
                surface_overlap.0 += line_surface_overlap;
            }
            RegionColor::Unknown => {
                ball_overlap.1 += line_ball_overlap;
                surface_overlap.1 += line_surface_overlap;
            }
        }
    }

    // fraction of the ball proposal that is ball-colored
    let ball_ratio = ball_overlap.0 / (ball_overlap.0 + ball_overlap.1);
    // fraction of the surface underneath the ball proposal that is grass-colored
    let grass_ratio = surface_overlap.0 / (surface_overlap.0 + surface_overlap.1);

    // abort if fractions do not equal or exceed the configured minimum requirements
    if ball_ratio < config.ball_ratio || grass_ratio < config.grass_ratio {
        return None;
    }

    let padded_radius = radius * config.bbox_padding_factor;
    let proposal_box = Bbox::xyxy(
        (ball_center.x - padded_radius).clamp(0.0, image_width),
        (ball_center.y - padded_radius).clamp(0.0, image_height),
        (ball_center.x + padded_radius).clamp(0.0, image_width),
        (ball_center.y + padded_radius).clamp(0.0, image_height),
    );

    let proposal = BallProposal {
        position: point![ball_center.x as usize, ball_center.y as usize],
        scale: radius * 2.0 * config.bbox_padding_factor,
        distance_to_ball: distance,
        bbox: proposal_box,
    };

    Some((proposal, (proposal_box, ball_ratio)))
}

#[must_use]
pub fn get_ball_proposals<T: CameraLocation>(
    scan_lines: &ScanLines<T>,
    matrix: &CameraMatrix<T>,
    config: &BallProposalConfig,
    body_contour: &BodyContour,
) -> BallProposals<T> {
    let h_lines = scan_lines.horizontal();

    let mut proposals = Vec::new();
    let mut detections = Vec::new();
    for (left, middle, right) in h_lines.regions().tuple_windows() {
        // check if scanlines are on same height
        if left.fixed_point() != middle.fixed_point() || middle.fixed_point() != right.fixed_point()
        {
            continue;
        }
        // check if a white scanline is surrounded by green scanlines
        let (RegionColor::Green, RegionColor::WhiteOrBlack, RegionColor::Green) =
            (left.color(), middle.color(), right.color())
        else {
            continue;
        };

        // middle of the white region
        let mid_point = middle.line_spot();

        // only check for body contours on bottom camera
        if T::POSITION == CameraPosition::Bottom && body_contour.is_part_of_body(mid_point) {
            continue;
        }

        let Ok(left) =
            matrix.pixel_to_ground(point![middle.start_point() as f32, mid_point.y], 0.0)
        else {
            continue;
        };

        let Ok(right) = matrix.pixel_to_ground(point![middle.end_point() as f32, mid_point.y], 0.0)
        else {
            continue;
        };

        // magnitude of the midpoint of left and right point
        let distance = (0.5 * (left.coords + right.coords)).magnitude();

        // diameter of the ball in meters
        let ball_diameter = (right - left).norm();
        let ball_radius = 0.5 * ball_diameter;

        if ball_radius < config.min_ball_radius || ball_radius > config.max_ball_radius {
            continue;
        }

        // find radius to look around the point,
        // bbox scale is diameter, so divide by 2 to get radius
        let scaling = config.bounding_box_scale * 0.5;
        let image_radius = scaling / distance;

        let image_size = (
            scan_lines.image().width() as f32,
            scan_lines.image().height() as f32,
        );

        // if the white line is long, divvy up white segment in multiple potential ball centers
        if middle.length() > (image_radius * 2.0 * config.ball_radius_max_error) as usize {
            // check point on left and right side of the white scanline
            let center = point![
                middle.start_point() as f32 + image_radius,
                middle.fixed_point() as f32
            ];
            if let Some((proposal, detection)) =
                check_proposal(center, image_radius, distance, h_lines, config, image_size)
            {
                proposals.push(proposal);
                detections.push(detection);
            }

            let center = point![
                middle.end_point() as f32 - image_radius,
                middle.fixed_point() as f32
            ];

            if let Some((proposal, detection)) =
                check_proposal(center, image_radius, distance, h_lines, config, image_size)
            {
                proposals.push(proposal);
                detections.push(detection);
            }
        }
        // otherwise only investigate center
        else if let Some((proposal, detection)) = check_proposal(
            mid_point,
            image_radius,
            distance,
            h_lines,
            config,
            image_size,
        ) {
            proposals.push(proposal);
            detections.push(detection);
        }
    }

    // apply non-max suppression
    let indices = crate::vision::util::non_max_suppression(&detections, config.nms_threshold);
    proposals = indices.iter().map(|&i| proposals[i].clone()).collect();

    BallProposals {
        image: scan_lines.image().clone(),
        proposals,
    }
}

fn init_ball_proposals<T: CameraLocation>(mut commands: Commands, image: Res<Image<T>>) {
    commands.insert_resource(BallProposals::empty(image.clone()));
}

fn log_ball_proposals<T: CameraLocation>(
    dbg: DebugContext,
    proposals: Res<BallProposals<T>>,
    cycle: Res<Cycle>,
) {
    if proposals.proposals.is_empty() {
        dbg.log_with_cycle(
            T::make_entity_image_path("balls/proposals"),
            *cycle,
            &rerun::Clear::flat(),
        );
        return;
    }
    let (mins, sizes): (Vec<_>, Vec<_>) = proposals
        .proposals
        .iter()
        .map(|proposal| {
            let (x1, y1, x2, y2) = proposal.bbox.inner;
            ((x1, y1), (x2 - x1, y2 - y1))
        })
        .unzip();

    dbg.log_with_cycle(
        T::make_entity_image_path("balls/proposals"),
        proposals.image.cycle(),
        &rerun::Boxes2D::from_mins_and_sizes(&mins, &sizes),
    );
}
