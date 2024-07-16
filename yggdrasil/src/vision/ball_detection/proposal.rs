//! Module for finding possible ball locations from the top camera image

use std::ops::Add;

use heimdall::CameraMatrix;
use itertools::Itertools;
use nalgebra::Point2;

use serde::{Deserialize, Serialize};

use crate::{
    core::debug::DebugContext,
    prelude::*,
    vision::{
        camera::{matrix::CameraMatrices, Image},
        scan_lines2::{
            self, BottomScanLines, CameraType, ClassifiedScanLineRegion, RegionColor, ScanLines,
            TopScanLines,
        },
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
    /// The minimum ratio of white pixels in the range around the proposed ball
    pub white_ratio: f32,
    /// Height/width of the bounding box around the ball
    pub bounding_box_scale: f32,
}

/// Module for finding possible ball locations in the top camera image
///
/// It adds the following resources to the app:
/// - [`TopBallProposals`]
/// - [`BottomBallProposals`]
pub struct BallProposalModule;

impl Module for BallProposalModule {
    fn initialize(self, app: App) -> Result<App> {
        app.add_system(ball_proposals_system.after(scan_lines2::scan_lines_system))
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
    dbg: &DebugContext,
) -> Result<()> {
    update_ball_proposals(
        top_proposals,
        top_scan_lines,
        &matrices.top,
        &config.top,
        CameraType::Top,
        dbg,
    )?;

    update_ball_proposals(
        bottom_proposals,
        bottom_scan_lines,
        &matrices.bottom,
        &config.bottom,
        CameraType::Bottom,
        dbg,
    )?;

    Ok(())
}

pub fn update_ball_proposals(
    ball_proposals: &mut BallProposals,
    scan_lines: &ScanLines,
    matrix: &CameraMatrix,
    config: &BallProposalConfig,
    camera: CameraType,
    dbg: &DebugContext,
) -> Result<()> {
    // if the image has not changed, we don't need to recalculate the proposals
    if ball_proposals
        .image
        .is_from_cycle(scan_lines.image().cycle())
    {
        return Ok(());
    }

    let new = get_ball_proposals(scan_lines, matrix, config, camera, dbg)?;

    *ball_proposals = new;

    Ok(())
}

#[derive(Debug, Default, Clone)]
struct WhiteCounter {
    white: f32,
    other: f32,
}

impl WhiteCounter {
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
                RegionColor::White => (
                    white + overlap(region.start_point(), region.end_point()),
                    other,
                ),
                _ => (
                    white,
                    other + overlap(region.start_point(), region.end_point()),
                ),
            });

        Self { white, other }
    }

    fn white_ratio(&self) -> f32 {
        if self.other == 0.0 && self.white == 0.0 {
            return 0.0;
        }

        self.white / (self.other + self.white)
    }
}

impl Add for WhiteCounter {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            other: self.other + rhs.other,
            white: self.white + rhs.white,
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
    camera: CameraType,
    dbg: &DebugContext,
) -> Result<BallProposals> {
    let h_lines = scan_lines.horizontal();

    let mut proposals = Vec::new();
    let mut boxes = Vec::new();
    let mut scores = Vec::new();
    for (left, middle, right) in h_lines.regions().tuple_windows() {
        // Check if the three scanlines have the same height
        if (left.fixed_point() != middle.fixed_point())
            || (middle.fixed_point() != right.fixed_point())
        {
            continue;
        }

        // Check if the white region is surrounded by green regions
        let (RegionColor::Green, RegionColor::White, RegionColor::Green) =
            (left.color(), middle.color(), right.color())
        else {
            continue;
        };

        // Middle of the white region
        let mid_point = middle.line_spot();

        let Ok(distance) = matrix
            .pixel_to_ground(mid_point, 0.0)
            .map(|p| p.coords.magnitude())
        else {
            continue;
        };

        // Find radius to look around the point
        let scaling = config.bounding_box_scale * 0.5;
        let range = scaling / distance;

        // Scan circularly around the ball to find:
        // - The average y position of the ball
        // - The ratio of white pixels in the region
        let (y_locations, color_count) = (-range as usize..=range as usize).fold(
            (Vec::new(), WhiteCounter::default()),
            |(mut valid_locs, count), y| {
                let x = (range * range - (y * y) as f32).sqrt();

                let same_y_regions = h_lines
                    .regions()
                    .filter(|region| region.fixed_point() == mid_point.y as usize + y);

                let new_count =
                    WhiteCounter::from_regions(mid_point.x - x, mid_point.x + x, same_y_regions);

                // only count the y location if there are actually white pixels in the region
                if new_count.white > 0.0 {
                    valid_locs.push(mid_point.y as usize + y);
                }

                (valid_locs, count + new_count)
            },
        );

        // not white enough
        if color_count.white_ratio() < config.white_ratio {
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
        boxes.push((
            new_mid_point.x - range,
            new_mid_point.y - range,
            new_mid_point.x + range,
            new_mid_point.y + range,
        ));
        scores.push((range, color_count.white_ratio()));
    }

    let (radii, white_ratios): (Vec<_>, Vec<_>) = scores.into_iter().unzip();

    let indices = nms(boxes, white_ratios, 0.5);
    // let indices = (0..boxes.len()).collect::<Vec<_>>();

    let proposals: Vec<_> = indices.iter().map(|&i| proposals[i].clone()).collect();
    let potential: Vec<_> = proposals
        .iter()
        .map(|p| (p.position.x as f32, p.position.y as f32))
        .collect();

    let camera_str = if matches!(camera, CameraType::Top) {
        "top_camera"
    } else {
        "bottom_camera"
    };

    dbg.log_points2d_for_image_with_radii(
        format!("{camera_str}/image/bruh"),
        &potential,
        scan_lines.image().cycle(),
        nidhogg::types::color::u8::WHITE,
        radii,
    )?;

    let image = scan_lines.image().clone();

    Ok(BallProposals { image, proposals })
}

fn nms(boxes: Vec<BBox>, scores: Vec<f32>, threshold: f32) -> Vec<usize> {
    let mut final_indices = Vec::new();

    println!("Began with {}", boxes.len());

    for i in 0..boxes.len() {
        let mut discard = false;
        for j in 0..boxes.len() {
            if i == j {
                continue;
            }

            let overlap = iou(&boxes[i], &boxes[j]);
            println!("iou: {}", overlap);
            let score_i = scores[i];
            let score_j = scores[j];

            if overlap > threshold && score_j > score_i {
                discard = true;
                break;
            }
        }

        if !discard {
            final_indices.push(i);
        }
    }

    println!("Ended with {}", final_indices.len());

    final_indices
}

type BBox = (f32, f32, f32, f32);

pub fn intersection(box1: &BBox, box2: &BBox) -> f32 {
    let x1 = box1.0.max(box2.0);
    let y1 = box1.1.max(box2.1);
    let x2 = box1.2.min(box2.2);
    let y2 = box1.3.min(box2.3);

    if x2 < x1 || y2 < y1 {
        0.0
    } else {
        (x2 - x1) * (y2 - y1)
    }
}

fn union(box1: &BBox, box2: &BBox) -> f32 {
    let area1 = (box1.2 - box1.0) * (box1.3 - box1.1);
    let area2 = (box2.2 - box2.0) * (box2.3 - box2.1);
    area1 + area2 - intersection(box1, box2)
}

pub fn iou(box1: &BBox, box2: &BBox) -> f32 {
    let intersect = intersection(box1, box2);
    let union = union(box1, box2);

    intersect / union
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
