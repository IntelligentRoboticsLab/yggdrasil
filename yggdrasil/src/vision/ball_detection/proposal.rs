//! Module for finding possible ball locations from the top camera image

use std::ops::Add;

use heimdall::CameraMatrix;
use itertools::Itertools;
use nalgebra::Point2;

use nidhogg::types::color;
use serde::{Deserialize, Serialize};

use crate::{
    core::debug::DebugContext,
    prelude::*,
    vision::{
        camera::{matrix::CameraMatrices, Image},
        robot_detection::RobotDetectionData,
        scan_lines::{
            self, BottomScanLines, CameraType, ClassifiedScanLineRegion, RegionColor, ScanLines,
            TopScanLines,
        },
        util::bbox::Bbox,
    },
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BallProposalConfigs {
    /// Top camera ball proposal configuration
    pub top: BallProposalConfig,
    /// Bottom camera ball proposal configuration
    pub bottom: BallProposalConfig,
}

/// Configurable values for getting ball proposals during the ball detection pipeline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BallProposalConfig {
    /// The minimum ratio of white or black pixels in the range around the proposed ball
    pub ball_ratio: f32,
    /// Height/width of the bounding box around the ball
    pub bounding_box_scale: f32,
    /// The minimum overlap ratio between for bounding boxes to be merged using non-maximum suppression
    pub nms_threshold: f32,
    /// The minimum radius of the proposed ball in pixels.
    pub min_ball_radius: f32,
    /// The maximum area of the intersection between a detected robot and a proposed ball in pixels.
    pub max_robot_intersection: f32,
}

/// Module for finding possible ball locations in the top camera image
///
/// It adds the following resources to the app:
/// - [`TopBallProposals`]
/// - [`BottomBallProposals`]
pub struct BallProposalModule;

impl Module for BallProposalModule {
    fn initialize(self, app: App) -> Result<App> {
        app.add_system_chain((
            ball_proposals_system.after(scan_lines::scan_lines_system),
            log_proposals,
        ))
        .add_startup_system(init_ball_proposals)
    }
}

#[derive(derive_more::Deref, derive_more::DerefMut)]
pub struct TopBallProposals(BallProposals);

#[derive(derive_more::Deref, derive_more::DerefMut)]
pub struct BottomBallProposals(BallProposals);

/// Points at which a ball may possibly be located
#[derive(Clone)]
pub struct BallProposals {
    pub image: Image,
    pub proposals: Vec<BallProposal>,
}

impl BallProposals {
    fn empty(image: Image) -> Self {
        Self {
            image,
            proposals: Vec::new(),
        }
    }
}

#[derive(Default, Clone)]
pub struct BallProposal {
    pub position: Point2<usize>,
    pub scale: f32,
    pub distance_to_ball: f32,
}

#[system]
pub(super) fn ball_proposals_system(
    (top_scan_lines, bottom_scan_lines): (&TopScanLines, &BottomScanLines),
    matrices: &CameraMatrices,
    config: &BallProposalConfigs,
    (top_proposals, bottom_proposals): (&mut TopBallProposals, &mut BottomBallProposals),
    robots: &RobotDetectionData,
) -> Result<()> {
    update_ball_proposals(
        top_proposals,
        top_scan_lines,
        &matrices.top,
        &config.top,
        Some(robots),
    )?;
    update_ball_proposals(
        bottom_proposals,
        bottom_scan_lines,
        &matrices.bottom,
        &config.bottom,
        None,
    )?;

    Ok(())
}

pub fn update_ball_proposals(
    ball_proposals: &mut BallProposals,
    scan_lines: &ScanLines,
    matrix: &CameraMatrix,
    config: &BallProposalConfig,
    robots: Option<&RobotDetectionData>,
) -> Result<()> {
    // if the image has not changed, we don't need to recalculate the proposals
    if ball_proposals
        .image
        .is_from_cycle(scan_lines.image().cycle())
    {
        return Ok(());
    }

    let mut new = get_ball_proposals(scan_lines, matrix, config)?;

    if let Some(robots) = robots {
        // remove proposals that are too close to robots
        let robot_bboxes = robots.detected.iter().map(|robot| robot.bbox).collect_vec();

        new.proposals.retain(|proposal| {
            robot_bboxes.iter().all(|bbox| {
                let proposal_bbox = Bbox::cxcywh(
                    proposal.position.x as f32,
                    proposal.position.y as f32,
                    proposal.scale,
                    proposal.scale,
                );

                bbox.intersection(&proposal_bbox) <= config.max_robot_intersection
            })
        });
    }
    *ball_proposals = new;

    Ok(())
}

#[derive(Debug, Default, Clone)]
struct BallColorCounter {
    ball_color: f32,
    other: f32,
}

impl BallColorCounter {
    fn from_regions<'a>(
        start: f32,
        end: f32,
        regions: impl Iterator<Item = &'a ClassifiedScanLineRegion>,
    ) -> Self {
        let overlap = |region_start: usize, region_end: usize| -> f32 {
            range_overlap((start, end), (region_start as f32, region_end as f32))
                .unwrap_or_default()
        };

        let (white, other) =
            regions.fold((0.0, 0.0), |(white, other), region| match region.color() {
                RegionColor::WhiteOrBlack => (
                    white + overlap(region.start_point(), region.end_point()),
                    other,
                ),
                _ => (
                    white,
                    other + overlap(region.start_point(), region.end_point()),
                ),
            });

        Self {
            ball_color: white,
            other,
        }
    }

    fn ball_ratio(&self) -> f32 {
        if self.other == 0.0 && self.ball_color == 0.0 {
            return 0.0;
        }

        self.ball_color / (self.other + self.ball_color)
    }
}

impl Add for BallColorCounter {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            other: self.other + rhs.other,
            ball_color: self.ball_color + rhs.ball_color,
        }
    }
}

fn range_overlap((start1, end1): (f32, f32), (start2, end2): (f32, f32)) -> Option<f32> {
    let start_overlap = start1.max(start2);
    let end_overlap = end1.min(end2);

    // Check if there is an overlap
    if start_overlap < end_overlap {
        Some(end_overlap - start_overlap)
    } else {
        None
    }
}

fn get_ball_proposals(
    scan_lines: &ScanLines,
    matrix: &CameraMatrix,
    config: &BallProposalConfig,
) -> Result<BallProposals> {
    let h_lines = scan_lines.horizontal();

    let mut proposals = Vec::new();
    let mut detections = Vec::new();
    for (left, middle, right) in h_lines.regions().tuple_windows() {
        // Check if the three scanlines have the same height
        if (left.fixed_point() != middle.fixed_point())
            || (middle.fixed_point() != right.fixed_point())
        {
            continue;
        }

        // Check if the white region is surrounded by green regions
        let (RegionColor::Green, RegionColor::WhiteOrBlack, RegionColor::Green) =
            (left.color(), middle.color(), right.color())
        else {
            continue;
        };

        // Middle of the white region
        let mid_point = middle.line_spot();

        // Distance to the ball
        let Ok(distance) = matrix
            .pixel_to_ground(mid_point, 0.0)
            .map(|p| p.coords.magnitude())
        else {
            continue;
        };

        // Find radius to look around the point
        // bbox scale is diameter, so divide by 2 to get radius
        let scaling = config.bounding_box_scale * 0.5;
        let radius = scaling / distance;

        if radius < config.min_ball_radius {
            continue;
        }

        // Scan circularly around the ball to find in the scanlines:
        // - The average y position of the ball
        // - The ratio of white pixels in the region
        let (y_locations, color_count) = (-radius as usize..=radius as usize).fold(
            (Vec::new(), BallColorCounter::default()),
            |(mut valid_locs, count), y| {
                let x = (radius * radius - (y * y) as f32).sqrt();

                let same_y_regions = h_lines
                    .regions()
                    .filter(|region| region.fixed_point() == mid_point.y as usize + y);

                let new_count = BallColorCounter::from_regions(
                    mid_point.x - x,
                    mid_point.x + x,
                    same_y_regions,
                );

                // only count the y location if there are actually white pixels in the region
                if new_count.ball_color > 0.0 {
                    valid_locs.push(mid_point.y as usize + y);
                }

                (valid_locs, count + new_count)
            },
        );

        if color_count.ball_ratio() < config.ball_ratio {
            continue;
        }

        let y_len = y_locations.len();
        let y_avg = y_locations.into_iter().sum::<usize>() as f32 / y_len as f32;

        let new_mid_point = Point2::new(mid_point.x, y_avg);

        let proposal = BallProposal {
            position: new_mid_point.map(|x| x as usize),
            scale: config.bounding_box_scale / distance,
            distance_to_ball: distance,
        };

        proposals.push(proposal);
        let proposal_box = Bbox::xyxy(
            new_mid_point.x - radius,
            new_mid_point.y - radius,
            new_mid_point.x + radius,
            new_mid_point.y + radius,
        );

        detections.push((proposal_box, color_count.ball_ratio()));
    }

    let indices = crate::vision::util::non_max_suppression(&detections, config.nms_threshold);

    let proposals: Vec<_> = indices.iter().map(|&i| proposals[i].clone()).collect();
    let image = scan_lines.image().clone();

    Ok(BallProposals { image, proposals })
}

#[startup_system]
fn init_ball_proposals(
    storage: &mut Storage,
    (top_scan_lines, bottom_scan_lines): (&TopScanLines, &BottomScanLines),
) -> Result<()> {
    let top = BallProposals::empty(top_scan_lines.image().clone());
    let bottom = BallProposals::empty(bottom_scan_lines.image().clone());

    storage.add_resource(Resource::new(TopBallProposals(top)))?;
    storage.add_resource(Resource::new(BottomBallProposals(bottom)))?;

    Ok(())
}

#[system]
fn log_proposals(
    (top_proposals, bottom_proposals): (&TopBallProposals, &BottomBallProposals),
    matrices: &CameraMatrices,
    config: &BallProposalConfigs,
    dbg: &DebugContext,
) -> Result<()> {
    log_proposals_single_camera(
        top_proposals,
        &matrices.top,
        &config.top,
        CameraType::Top,
        dbg,
    )?;
    log_proposals_single_camera(
        bottom_proposals,
        &matrices.bottom,
        &config.bottom,
        CameraType::Bottom,
        dbg,
    )?;

    Ok(())
}

fn log_proposals_single_camera(
    ball_proposals: &BallProposals,
    matrix: &CameraMatrix,
    config: &BallProposalConfig,
    camera: CameraType,
    dbg: &DebugContext,
) -> Result<()> {
    let camera_str = match camera {
        CameraType::Top => "top_camera",
        CameraType::Bottom => "bottom_camera",
    };

    let mut points = Vec::new();
    let mut sizes = Vec::new();
    for proposal in &ball_proposals.proposals {
        // project point to ground to get distance
        // distance is used for the amount of surrounding pixels to sample
        let Ok(coord) = matrix.pixel_to_ground(proposal.position.cast(), 0.0) else {
            continue;
        };

        let magnitude = coord.coords.magnitude();

        let size = config.bounding_box_scale / magnitude;

        points.push((proposal.position.x as f32, proposal.position.y as f32));
        sizes.push((size, size));
    }

    dbg.log_boxes_2d(
        format!("{camera_str}/image/ball_boxes"),
        &points,
        &sizes,
        &ball_proposals.image,
        color::u8::SILVER,
    )?;

    Ok(())
}
