use crate::vision::camera::matrix::CameraMatrices;
use crate::vision::line_detection::TopLines;
use crate::vision::scan_lines::TopScanLines;
use crate::{core::debug::DebugContext, prelude::*};

use lstsq::Lstsq;
use nalgebra::{distance, DVector, Matrix2, Point2, Point3, Vector2};
use nidhogg::types::color;

const MAX_DISTANCE: f32 = 0.75;

pub struct MiddleCircleDetectionModule;

impl Module for MiddleCircleDetectionModule {
    fn initialize(self, app: App) -> Result<App> {
        Ok(app
            .init_resource::<MiddleCircleDetector>()?
            .add_system(test))
    }
}

#[derive(Default, Clone, Debug)]
struct MiddleCircleCandidate {
    center: Point2<f32>,
    radius: f32,
    line_spots: Vec<Point2<f32>>,
}

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

impl MiddleCircleCandidate {
    fn add_spot(&mut self, spot: Point2<f32>) {
        self.line_spots.push(spot);
        self.update_fit()
    }

    fn update_fit(&mut self) {
        let (new_center, new_radius) = self.fit();
        self.center = new_center;
        self.radius = new_radius;
    }

    /// This algorithm follows the following paper:
    /// https://dtcenter.org/sites/default/files/community-code/met/docs/write-ups/circle_fit.pdf
    fn fit(&self) -> (Point2<f32>, f32) {
        let (x, y) = points_to_vectors(&self.line_spots);

        let x_m = x.mean();
        let y_m = y.mean();

        let u = x.map(|elem| elem - x_m);
        let v = y.map(|elem| elem - y_m);

        let s_uu = u.map(|elem| elem * elem).sum();
        let s_vv = v.map(|elem| elem * elem).sum();

        let vs_uv = u.component_mul(&v);
        let s_uv = vs_uv.sum();

        let s_uuu: f32 = u.map(|elem| elem * elem * elem).sum();
        let s_vvv: f32 = v.map(|elem| elem * elem * elem).sum();
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

        let x_c = u_c + x_m;
        let y_c = v_c + y_m;

        let n = self.line_spots.len() as f32;
        let a = u_c.powi(2) + v_c.powi(2) + ((s_uu + s_vv) / n);
        let radius = a.sqrt();

        (Point2::new(x_c, y_c), radius)
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
        for candidate in &mut middle_circle_detector.candidates {
            if distance(&candidate.center, &spot) > MAX_DISTANCE {
                candidate.add_spot(spot);
            }
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
