//! See [`BallProposalPlugin`].

use bevy::prelude::*;
use std::ops::Add;

use heimdall::{CameraLocation, CameraMatrix, CameraPosition};
use itertools::Itertools;
use nalgebra::Point2;

use serde::{Deserialize, Serialize};

use crate::{
    core::debug::DebugContext,
    nao::Cycle,
    vision::{
        body_contour::BodyContour,
        camera::{init_camera, Image},
        scan_lines::{ClassifiedScanLineRegion, RegionColor, ScanLines},
        util::bbox::Bbox,
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
    /// Height/width of the bounding box around the ball
    pub bounding_box_scale: f32,
    /// The minimum overlap ratio between for bounding boxes to be merged using non-maximum suppression
    pub nms_threshold: f32,
    /// The minimum radius of the proposed ball in pixels.
    pub min_ball_radius: f32,
    /// The maximum area of the intersection between a detected robot and a proposed ball in pixels.
    pub max_robot_intersection: f32,
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
                (update_ball_proposals::<T>, log_ball_proposals::<T>)
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

#[derive(Default, Clone)]
pub struct BallProposal {
    pub position: Point2<usize>,
    pub scale: f32,
    pub distance_to_ball: f32,
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

fn get_ball_proposals<T: CameraLocation>(
    scan_lines: &ScanLines<T>,
    matrix: &CameraMatrix<T>,
    config: &BallProposalConfig,
    body_contour: &BodyContour,
) -> BallProposals<T> {
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

        if body_contour.is_part_of_body(mid_point) {
            continue;
        }

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

    BallProposals { image, proposals }
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
    let (positions, half_sizes): (Vec<_>, Vec<_>) = proposals
        .proposals
        .iter()
        .map(|proposal| {
            (
                (proposal.position.x as f32, proposal.position.y as f32),
                (proposal.scale / 2.0, proposal.scale / 2.0),
            )
        })
        .unzip();

    dbg.log_with_cycle(
        T::make_entity_image_path("balls/proposals"),
        proposals.image.cycle(),
        &rerun::Boxes2D::from_centers_and_half_sizes(&positions, &half_sizes),
    );
}
