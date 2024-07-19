use crate::vision::camera::matrix::CameraMatrices;
use crate::vision::line_detection::TopLines;
use crate::vision::scan_lines::TopScanLines;
use crate::{core::debug::DebugContext, prelude::*};

use lstsq::Lstsq;
use nalgebra::{distance, DVector, Matrix2, Point2, Point3, SymmetricEigen, Vector2};
use nidhogg::types::color;

const MAX_CIRCLE_FITTING_ERROR: f32 = 100.0;
const MIN_SPOTS_ON_CIRCLE: u32 = 10;

pub struct MiddleCircleDetectionModule;

impl Module for MiddleCircleDetectionModule {
    fn initialize(self, app: App) -> Result<App> {
        Ok(app
            .init_resource::<MiddleCircleDetector>()?
            .add_system(test))
    }
}

/// Converts a vector of 2d points to two seperate nalgbra vectors of
/// coordinates
fn points_to_vectors(points: &[Point2<f32>]) -> (DVector<f32>, DVector<f32>) {
    let n = points.len();
    let mut x_coords = Vec::with_capacity(n);
    let mut y_coords = Vec::with_capacity(n);

    for point in points {
        x_coords.push(point.x);
        y_coords.push(point.y);
    }

    let x_vector = DVector::<f32>::from_vec(x_coords);
    let y_vector = DVector::<f32>::from_vec(y_coords);

    (x_vector, y_vector)
}

/// Line representation that uses a normal to the line
struct Line {
    /// Normal to the line itself
    normal: Vector2<f32>,
    /// Distance to the origin
    d: f32,
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
    pub fn fit(points: &[Point2<f32>]) -> Line {
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

        let smallest_eig_idx = if eig.eigenvalues[0] > eig.eigenvalues[1] {
            0
        } else {
            1
        };

        // Select smallest eigenvector as the norm (smallest variance)
        let norm = eig.eigenvectors.column(smallest_eig_idx);

        // Distance to the origin
        let d = norm.dot(&Vector2::new(x_m, y_m));

        Line {
            normal: norm.into(),
            d,
        }
    }
}

/// Representation of a circle
#[derive(Default, Clone, Debug)]
struct Circle {
    /// Circle center
    center: Point2<f32>,
    /// Circle radius
    radius: f32,
}

impl Circle {
    /// Computes the distance between the middle of a circle to a point
    pub fn distance_to_point(&self, point: &Point2<f32>) -> f32 {
        (self.center.coords - point.coords).norm()
    }

    /// This algorithm follows the following paper:
    /// https://dtcenter.org/sites/default/files/community-code/met/docs/write-ups/circle_fit.pdf
    /// It formulates the circle fitting problem as a least squares problem and
    /// computes a center and radius
    pub fn fit(points: &[Point2<f32>]) -> Circle {
        // Convert the points to nalgebra vectors to make operations easier
        let (x, y) = points_to_vectors(points);

        let x_m = x.mean();
        let y_m = y.mean();

        // Normalized x and y coordinates
        let u = x.map(|elem| elem - x_m);
        let v = y.map(|elem| elem - y_m);

        let s_uu = u.map(|elem| elem.powi(2)).sum();
        let s_vv = v.map(|elem| elem.powi(2)).sum();

        let vs_uv = u.component_mul(&v);
        let s_uv = vs_uv.sum();

        let s_uuu: f32 = u.map(|elem| elem.powi(3)).sum();
        let s_vvv: f32 = v.map(|elem| elem.powi(3)).sum();
        let s_uvv: f32 = vs_uv.dot(&v);
        let s_vuu: f32 = vs_uv.dot(&u);

        let a = Matrix2::<f32>::new(s_uu, s_uv, s_uv, s_vv);
        let b = Vector2::<f32>::new(0.5 * (s_uuu + s_uvv), 0.5 * (s_vvv + s_vuu));

        // use same epsilon as the numpy implementation
        let epsilon = f32::EPSILON * a.nrows().max(a.ncols()) as f32;

        // fit using least squares
        let Lstsq { solution, .. } = lstsq::lstsq(&a, &b, epsilon).unwrap();
        let u_c = solution[0];
        let v_c = solution[1];

        // Compute center
        let x_c = u_c + x_m;
        let y_c = v_c + y_m;
        let center = Point2::new(x_c, y_c);

        let n = points.len() as f32;
        let a = u_c.powi(2) + v_c.powi(2) + ((s_uu + s_vv) / n);
        let radius = a.sqrt();

        Circle { center, radius }
    }
}

/// Middle circle candidate, this candidate contains a fitted circle and the
/// spots on which the circle was fitted. Spots are added iteratively and each
/// time a spot is added the fit is updated.
#[derive(Default, Clone, Debug)]
struct MiddleCircleCandidate {
    /// The currently fitted circle
    fitted_circle: Circle,
    /// Line spots on which the current circle candidate is fitted
    line_spots: Vec<Point2<f32>>,
}

impl MiddleCircleCandidate {
    /// Adds a new spot the points used for fitting this circle candidate
    fn add_spot(&mut self, spot: Point2<f32>) {
        self.line_spots.push(spot);
        self.update_fit()
    }

    /// Fits a new circle and uses this to update the current fit
    fn update_fit(&mut self) {
        self.fitted_circle = Circle::fit(&self.line_spots);
    }
}

#[derive(Default, Clone, Debug)]
pub struct MiddleCircleDetector {
    candidates: Vec<MiddleCircleCandidate>,
}

#[system]
pub fn test(
    dbg: &DebugContext,
    camera_matrices: &CameraMatrices,
    top_lines: &TopLines,
    top_scan_lines: &TopScanLines,
    middle_circle_detector: &mut MiddleCircleDetector,
) -> Result<()> {
    for spot in top_scan_lines.horizontal().line_spots() {
        let mut circle_fitted = false;

        for candidate in &mut middle_circle_detector.candidates {
            if distance(&candidate.fitted_circle.center, &spot) <= MAX_CIRCLE_FITTING_ERROR {
                candidate.add_spot(spot);
                circle_fitted = true;
                break;
            }
        }

        if !circle_fitted {
            println!("asdf");
        }
    }

    let projected_lines: Vec<(Point3<f32>, Point3<f32>)> = top_lines
        .0
        .iter()
        .filter_map(|line| {
            let projected_start = camera_matrices.top.pixel_to_ground(line.start, 0.0).ok();
            let projected_end = camera_matrices.top.pixel_to_ground(line.end, 0.0).ok();

            projected_start.zip(projected_end)
        })
        .collect();

    let center = (0.0, 0.0);
    let radius = 0.75;
    dbg.log_lines3d("field/mesh/lines", &projected_lines, color::u8::BLUE)?;
    dbg.log_circle3d("field/mesh/middle_circle", center, radius, color::u8::RED)?;

    Ok(())
}
