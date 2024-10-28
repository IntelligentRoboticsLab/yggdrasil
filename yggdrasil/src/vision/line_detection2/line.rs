use heimdall::{CameraLocation, CameraMatrix};
use itertools::Itertools;
use nalgebra::{Point2, Vector2};

use crate::vision::camera::Image;

use super::{ground_to_pixel, is_less_bright_and_more_saturated};

#[derive(Debug, Clone)]
pub struct Line2 {
    /// Normal to the line itself
    pub normal: Vector2<f32>,
    /// Distance to the origin
    pub d: f32,
}

impl Line2 {
    pub fn y(&self, x: f32) -> f32 {
        (-self.d - self.normal.x * x) / self.normal.y
    }
}

#[derive(Debug, Clone)]
pub struct LineSegment2 {
    pub start: Point2<f32>,
    pub end: Point2<f32>,
}

impl LineSegment2 {
    pub fn new(start: Point2<f32>, end: Point2<f32>) -> Self {
        Self { start, end }
    }

    pub fn length(&self) -> f32 {
        (self.end - self.start).norm()
    }

    pub fn center(&self) -> Point2<f32> {
        (self.start + self.end.coords) / 2.0
    }

    // get normal vector of the line segment
    pub fn normal(&self) -> Vector2<f32> {
        let dir = self.end - self.start;
        Vector2::new(-dir.y, dir.x).normalize()
    }

    // samples n points uniformly *in between* the two endpoints (excluding the endpoints themselves).
    pub fn sample_uniform(&self, n: usize) -> impl Iterator<Item = Point2<f32>> + use<'_> {
        (1..=n).map(move |i| {
            let t = i as f32 / (n + 1) as f32;
            self.start + (self.end - self.start) * t
        })
    }

    pub(crate) fn to_rerun_2d(&self) -> [(f32, f32); 2] {
        [(self.start.x, self.start.y), (self.end.x, self.end.y)]
    }

    pub(crate) fn to_rerun_3d(&self) -> [(f32, f32, f32); 2] {
        [
            (self.start.x, self.start.y, 0.0),
            (self.end.x, self.end.y, 0.0),
        ]
    }

    pub fn white_test<T: CameraLocation>(
        &self,
        image: &Image<T>,
        camera_matrix: &CameraMatrix<T>,
        n: usize,
        sample_distance: f32,
        ratio: f32,
    ) -> bool {
        let mut tests = vec![];

        for sample in self.sample_uniform(n) {
            let normal = self.normal();

            let tester1 = sample + normal * sample_distance;
            let tester2 = sample - normal * sample_distance;

            // project the points back to the image
            let (point_pixel, tester1_pixel, tester2_pixel) = (
                ground_to_pixel(&camera_matrix, sample).unwrap(),
                ground_to_pixel(&camera_matrix, tester1).unwrap(),
                ground_to_pixel(&camera_matrix, tester2).unwrap(),
            );

            let test1 = is_less_bright_and_more_saturated(tester1_pixel, point_pixel, image);
            let test2 = is_less_bright_and_more_saturated(tester2_pixel, point_pixel, image);

            tests.extend([test1, test2]);
        }

        let n_tests = tests.len() as f32;
        let n_true = tests.iter().filter(|&&x| x).count() as f32;

        n_true / n_tests > ratio
    }
}

#[derive(Debug, Clone)]
pub struct LineCandidate {
    pub line: Line2,
    pub inliers: Vec<Point2<f32>>,
    pub segment: LineSegment2,
}

impl LineCandidate {
    pub fn new(line: Line2, mut inliers: Vec<Point2<f32>>) -> Self {
        assert!(inliers.len() >= 2);
        Self::sort_inliers(&mut inliers);

        let segment = LineSegment2::new(*inliers.first().unwrap(), *inliers.last().unwrap());

        Self {
            line,
            inliers,
            segment,
        }
    }

    // Create a LineCandidate without sorting the inliers or checking the length
    fn new_unchecked(line: Line2, inliers: Vec<Point2<f32>>) -> Self {
        let segment = LineSegment2::new(*inliers.first().unwrap(), *inliers.last().unwrap());

        Self {
            line,
            inliers,
            segment,
        }
    }

    pub fn merge(&mut self, other: LineCandidate) {
        self.inliers.extend(other.inliers);
        Self::sort_inliers(&mut self.inliers);
        self.segment = LineSegment2::new(
            *self.inliers.first().unwrap(),
            *self.inliers.last().unwrap(),
        );
    }

    pub fn n_inliers(&self) -> usize {
        self.inliers.len()
    }

    /// Split the line candidate into multiple candidates if the gap between two neighboring inliers is too large
    ///
    /// Returns a list of new line candidates and the points that are not included in any of the candidates
    pub fn split_at_gap(self, max_gap: f32) -> (Vec<LineCandidate>, Vec<Point2<f32>>) {
        let mut candidates = vec![];
        let mut remaining_points = vec![];

        let mut curr_inliers = vec![];
        for (p1, p2) in self.inliers.iter().copied().tuple_windows() {
            if nalgebra::distance(&p1, &p2) > max_gap {
                if curr_inliers.len() >= 2 {
                    candidates.push(LineCandidate::new_unchecked(
                        self.line.clone(),
                        curr_inliers,
                    ));
                } else {
                    // if there are less than 2 points, add them to the remaining points
                    remaining_points.extend(curr_inliers);
                }
                curr_inliers = vec![];
            } else {
                curr_inliers.push(p1);
            }
        }

        // remainder
        if curr_inliers.len() >= 2 {
            candidates.push(LineCandidate::new_unchecked(self.line, curr_inliers));
        } else {
            remaining_points.extend(curr_inliers);
        }

        (candidates, remaining_points)
    }

    fn sort_inliers(inliers: &mut Vec<Point2<f32>>) {
        inliers.sort_unstable_by(|a, b| a.x.partial_cmp(&b.x).unwrap());
    }
}
