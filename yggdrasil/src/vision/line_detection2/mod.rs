pub mod arrsac;

use bevy::prelude::*;
use heimdall::{CameraLocation, CameraMatrix, Top, YuyvImage};
use nalgebra::{point, DVector, Matrix2, Point2, SymmetricEigen, Vector2};
use rand::Rng;

use super::{camera::Image, scan_lines::ScanLines};
use crate::core::debug::DebugContext;

const LINE_SEGMENT_MIN_POINTS: usize = 4;
const MAX_LINE_FIT_DISTANCE: f32 = 0.1;
const WHITE_TEST_SAMPLE_DISTANCE: f32 = 0.08;

/// Plugin that adds systems to detect lines from scan-lines.
pub struct LineDetectionPlugin;

impl Plugin for LineDetectionPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            detect_lines::<Top>.run_if(resource_exists_and_changed::<ScanLines<Top>>),
        );
    }
}

/// Line representation that uses a normal to the line
struct Line {
    /// Normal to the line itself
    normal: Vector2<f32>,
    /// Distance to the origin
    d: f32,
}

// samples n points uniformly *in between* p1 and p2.
fn uniform_between(
    p1: Point2<f32>,
    p2: Point2<f32>,
    n: usize,
) -> impl Iterator<Item = Point2<f32>> {
    (1..=n).map(move |i| {
        let t = i as f32 / (n + 1) as f32;
        p1 + (p2 - p1) * t
    })
}

impl Line {
    /// Computes the distance from a line to a point. This is done as follows:
    /// first the vector to the point is projected onto the normalized normal
    /// of the line. This is done by using the dot product between the
    /// coordinates of the point and the normal of the line, which gives
    /// us the length of the projection. We can then subtract that from the
    /// total distance to the normal to get the remaining part, namely the
    /// distance between the point and the line.
    pub fn distance_to_point(&self, point: Point2<f32>) -> f32 {
        self.d - self.normal.dot(&point.coords)
    }

    /// Fits a line that uses a normal in its representation by taking the
    /// eigenvector with smallest corresponding eigenvalue.
    pub fn fit(points: impl Iterator<Item = Point2<f32>>) -> Line {
        // Convert the points to nalgebra vectors to make operations easier
        let (x, y) = points_to_vectors(points);

        let x_m = x.mean();
        let y_m = y.mean();

        // Normalized x and y coordinates
        let u = x.map(|elem| elem - x_m);
        let v = y.map(|elem| elem - y_m);

        let s_uu = u.map(|elem| elem.powi(2)).sum();
        let s_vv = v.map(|elem| elem.powi(2)).sum();

        let s_uv = u.component_mul(&v).sum();

        let a = Matrix2::<f32>::new(s_vv, s_uv, s_uv, s_uu);
        let eig = SymmetricEigen::new(a);

        let smallest_eig_idx = if eig.eigenvalues[0] < eig.eigenvalues[1] {
            0
        } else {
            1
        };

        // Select smallest eigenvector as the norm (smallest variance)
        let normal: Vector2<f32> = eig.eigenvectors.column(smallest_eig_idx).into();

        // Distance to the origin
        let d = normal.dot(&Vector2::new(x_m, y_m));

        Line { normal, d }
    }

    // /// Mean error of the line to a set of points
    // pub fn mean_error(&self, points: &[Point2<f32>]) -> f32 {
    //     points
    //         .iter()
    //         .map(|p| self.distance_to_point(*p).abs())
    //         .sum::<f32>()
    //         / points.len() as f32
    // }

    pub fn remove_outliers(
        &self,
        points: Vec<Point2<f32>>,
    ) -> (Vec<Point2<f32>>, Vec<Point2<f32>>) {
        // calculates both in one function to avoid recalculating the distance
        let distances_abs = points
            .iter()
            .map(|p| self.distance_to_point(*p).abs())
            .collect::<Vec<f32>>();

        let mean = distances_abs.iter().sum::<f32>() / distances_abs.len() as f32;

        let std_dev = distances_abs
            .iter()
            .map(|d| (d - mean).powi(2))
            .sum::<f32>()
            .sqrt()
            / distances_abs.len() as f32;

        let (good, bad): (Vec<_>, Vec<_>) = points
            .into_iter()
            .zip(distances_abs)
            // outliers are those that are more than 2 standard deviations away from the mean
            .partition(|(_, d)| *d < mean + 1.0 * std_dev);

        (
            good.into_iter().map(|(p, _)| p).collect(),
            bad.into_iter().map(|(p, _)| p).collect(),
        )
    }
}

pub fn is_less_light_and_more_saturated<T: CameraLocation>(
    p1: Point2<f32>,
    p2: Point2<f32>,
    image: &Image<T>,
) -> bool {
    #[inline]
    fn yhs_triple(p: Point2<f32>, image: &YuyvImage) -> Option<(f32, f32, f32)> {
        let (x, y) = (p.x as usize, p.y as usize);
        let pixel = image.pixel(x, y)?;
        Some(pixel.to_yhs2())
    }

    let Some((y1, _h1, s1)) = yhs_triple(p1, image) else {
        return false;
    };

    let Some((y2, _h2, s2)) = yhs_triple(p2, image) else {
        return false;
    };

    y1 > y2 && s1 < s2
}

fn remove_bad_fitting_points(
    points: Vec<Point2<f32>>,
    max_fit_distance: f32,
) -> (Vec<Point2<f32>>, Vec<Point2<f32>>) {
    let line = Line::fit(points.iter().copied());
    let mut good = vec![];
    let mut bad = vec![];

    for point in points {
        if line.distance_to_point(point).abs() < max_fit_distance {
            good.push(point);
        } else {
            bad.push(point);
        }
    }

    (good, bad)
}

fn detect_lines<T: CameraLocation>(
    dbg: DebugContext,
    scan_lines: Res<ScanLines<T>>,
    camera_matrix: Res<CameraMatrix<T>>,
) {
    let mut all_groups = scan_lines.vertical().line_spot_groups();
    all_groups.extend(scan_lines.horizontal().line_spot_groups());

    let mut discards = vec![];
    let mut filtered_groups = vec![];
    for group in all_groups {
        // // TODO: this needs to be after projection
        let projected = group
            .iter()
            .cloned()
            .flat_map(|p| pixel_to_ground(&camera_matrix, p))
            .collect::<Vec<_>>();
        // let (good, bad) = remove_bad_fitting_points(projected, MAX_LINE_FIT_DISTANCE);
        // discards.extend(bad);

        // if good.len() >= LINE_SEGMENT_MIN_POINTS {
        //     filtered_groups.push(good);
        // } else {
        //     discards.extend(good);
        // }

        let (good, bad) = Line::fit(projected.iter().copied()).remove_outliers(projected);
        discards.extend(bad);
        if good.len() >= LINE_SEGMENT_MIN_POINTS {
            filtered_groups.push(good);
        } else {
            discards.extend(good);
        };
    }

    let n_groups = filtered_groups.len();
    let mut dbg_all_pts = vec![];
    let mut dbg_all_colors = vec![];
    let mut dbg_all_radii = vec![];
    for i in (0..n_groups).rev() {
        for j in 0..i {
            let g1 = &filtered_groups[i];
            let g2 = &filtered_groups[j];

            let line = Line::fit(g1.iter().chain(g2.iter()).copied());

            // TODO: take sample n based on projected distance
            let samples = uniform_between(g1[0], g2[0], 10).collect::<Vec<_>>();

            // let mean_error = line.mean_error(&samples);

            let mut pts = vec![];
            let mut colors = vec![];
            let mut tests = vec![];
            let mut radii = vec![];

            for sample in samples {
                // TODO: use config and projections

                let p1 = sample + Vector2::new(0.0, WHITE_TEST_SAMPLE_DISTANCE);
                let p2 = sample + Vector2::new(0.0, -WHITE_TEST_SAMPLE_DISTANCE);

                let (test1, test2) = if let Some(sample) = ground_to_pixel(&camera_matrix, sample) {
                    let test1 = if let Some(p1) = ground_to_pixel(&camera_matrix, p1) {
                        is_less_light_and_more_saturated(sample, p1, scan_lines.image())
                    } else {
                        false
                    };
                    let test2 = if let Some(p2) = ground_to_pixel(&camera_matrix, p2) {
                        is_less_light_and_more_saturated(sample, p2, scan_lines.image())
                    } else {
                        false
                    };

                    (test1, test2)
                } else {
                    (false, false)
                };

                pts.extend([sample, p1, p2]);
                colors.extend([
                    rerun::Color::from_rgb(0, 0, 255),
                    if test1 {
                        rerun::Color::from_rgb(0, 255, 0)
                    } else {
                        rerun::Color::from_rgb(255, 0, 0)
                    },
                    if test2 {
                        rerun::Color::from_rgb(0, 255, 0)
                    } else {
                        rerun::Color::from_rgb(255, 0, 0)
                    },
                ]);
                tests.extend([test1, test2]);
                radii.extend([rerun::Radius(2.0.into()); 3]);
            }

            let ratio = tests.iter().filter(|&&t| t).count() as f32 / tests.len() as f32;

            // DEBUG REMOVE
            if ratio > 0.85 {
                dbg_all_pts.extend(pts.clone());
                dbg_all_colors.extend(colors.clone());
                dbg_all_radii.extend(radii.clone());
            }

            // if ratio > 0.85 && mean_error < MAX_LINE_FIT_DISTANCE {
            if ratio > 0.85 {
                // println!("Ratio: {}, Mean error: {}", ratio, mean_error);
                let to_add = filtered_groups.remove(i);
                filtered_groups[j].extend(to_add);

                break;
            }

            // dbg.log_with_cycle(
            //     T::make_entity_path(format!("spot_groups/extended")),
            //     scan_lines.image().cycle(),
            //     &rerun::Points2D::new(pts.iter().map(|p| (p.x, p.y)).collect::<Vec<_>>())
            //         .with_colors(colors)
            //         .with_radii(radii),
            // );
        }
    }
    dbg.log_with_cycle(
        T::make_entity_path(format!("spot_groups/extended")),
        scan_lines.image().cycle(),
        &rerun::Points2D::new(
            dbg_all_pts
                .iter()
                .flat_map(|p| ground_to_pixel(&camera_matrix, *p))
                .map(|p| p.to_tuple())
                .collect::<Vec<_>>(),
        )
        .with_colors(dbg_all_colors)
        .with_radii(dbg_all_radii),
    );

    // logging the groups
    let mut rng = rand::thread_rng();

    let mut groups = vec![];
    let mut colors = vec![];
    let mut radii = vec![];
    let mut lines = vec![];
    for group in filtered_groups {
        let mut group = group
            .into_iter()
            .flat_map(|p| ground_to_pixel(&camera_matrix, p))
            .map(|p| p.to_tuple())
            .collect::<Vec<_>>();
        let (r, g, b) = (rng.gen(), rng.gen(), rng.gen());
        let mut color = vec![rerun::Color::from_rgb(r, g, b); group.len()];
        let mut radius = vec![rerun::Radius(2.0.into()); group.len()];

        // get line segment from group and fitted line
        let (x1, y1) = group.first().unwrap().clone();
        let (x2, y2) = group.last().unwrap().clone();

        let mut line = vec![[(x1, y1), (x2, y2)]];

        groups.append(&mut group);
        colors.append(&mut color);
        radii.append(&mut radius);
        lines.append(&mut line);
    }

    dbg.log_with_cycle(
        T::make_entity_path(format!("spot_groups/vertical")),
        scan_lines.image().cycle(),
        &rerun::Points2D::new(groups)
            .with_colors(colors)
            .with_radii(radii),
    );

    dbg.log_with_cycle(
        T::make_entity_path(format!("spot_groups/lines")),
        scan_lines.image().cycle(),
        &rerun::LineStrips2D::new(lines.clone())
            .with_colors(vec![rerun::Color::from_rgb(255, 0, 0); lines.len()]),
    );

    let discards_proj = discards
        .clone()
        .into_iter()
        .map(|p| {
            let (x, y) = p.to_tuple();
            (x, y, 0.0)
        })
        .collect::<Vec<_>>();

    let discards = discards
        .into_iter()
        .flat_map(|p| ground_to_pixel(&camera_matrix, p))
        .map(|p| p.to_tuple())
        .collect::<Vec<_>>();
    let colors1 = vec![rerun::Color::from_rgb(255, 0, 255); discards.len()];
    let colors = vec![rerun::Color::from_rgb(255, 0, 0); discards.len()];
    let radius1 = vec![rerun::Radius(0.10.into()); discards.len()];
    let radius = vec![rerun::Radius(2.0.into()); discards.len()];

    dbg.log_with_cycle(
        T::make_entity_path("/projected_discards"),
        scan_lines.image().cycle(),
        &rerun::Points3D::new(discards_proj)
            .with_colors(colors1)
            .with_radii(radius1),
    );

    dbg.log_with_cycle(
        T::make_entity_path(format!("spot_groups/discards")),
        scan_lines.image().cycle(),
        &rerun::Points2D::new(discards)
            .with_colors(colors)
            .with_radii(radius),
    );
}

/// Converts a vector of 2d points to two seperate nalgbra vectors of
/// coordinates
fn points_to_vectors(points: impl Iterator<Item = Point2<f32>>) -> (DVector<f32>, DVector<f32>) {
    let (x, y): (Vec<f32>, Vec<f32>) = points.map(|p| (p.x, p.y)).unzip();
    (DVector::<f32>::from_vec(x), DVector::<f32>::from_vec(y))
}

fn pixel_to_ground<T: CameraLocation>(
    camera_matrix: &CameraMatrix<T>,
    point: Point2<f32>,
) -> Option<Point2<f32>> {
    let ground = camera_matrix.pixel_to_ground(point, 0.0).ok()?;
    Some(ground.xy())
}

fn ground_to_pixel<T: CameraLocation>(
    camera_matrix: &CameraMatrix<T>,
    point: Point2<f32>,
) -> Option<Point2<f32>> {
    let camera = camera_matrix
        .ground_to_pixel(point![point.x, point.y, 0.0])
        .ok()?;
    Some(camera)
}

trait ToTuple {
    type Output;

    fn to_tuple(&self) -> Self::Output;
}

impl ToTuple for Point2<f32> {
    type Output = (f32, f32);

    fn to_tuple(&self) -> Self::Output {
        (self.x, self.y)
    }
}
