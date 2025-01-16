use nalgebra::Point2;
use rand::prelude::{SliceRandom, ThreadRng};

use crate::vision::line_detection::line::{Line2, LineSegment2};

use super::Ransac;

/// Detects lines in a set of points using the RANSAC algorithm.
pub struct LineDetector {
    rng: ThreadRng,
    unused_points: Vec<Point2<f32>>,
    iterations: usize,
    inlier_threshold: f32,
}

impl LineDetector {
    #[must_use]
    pub fn new(
        unused_points: Vec<Point2<f32>>,
        iterations: usize,
        inlier_threshold: f32,
    ) -> LineDetector {
        let rng = rand::thread_rng();

        LineDetector {
            rng,
            unused_points,
            iterations,
            inlier_threshold,
        }
    }
}

impl Ransac for LineDetector {
    type Model = Line2;
    type Data = Point2<f32>;

    const MIN_SAMPLES: usize = 2;

    fn next(&mut self) -> Option<(Self::Model, Vec<Self::Data>)> {
        if self.unused_points.len() < Self::MIN_SAMPLES {
            return None;
        }

        let lines = (0..self.iterations)
            .map(|_| {
                let mut points = self
                    .unused_points
                    .choose_multiple(&mut self.rng, Self::MIN_SAMPLES);

                let line = LineSegment2::new(
                    points.next().copied().unwrap(),
                    points.next().copied().unwrap(),
                )
                .to_line();

                let score: f32 = self
                    .unused_points
                    .iter()
                    .map(|point| line.distance_to_point(*point))
                    .filter(|&distance| distance <= self.inlier_threshold)
                    // HULKs score function
                    .map(|distance| 1.0 - distance / self.inlier_threshold)
                    .sum();

                (line, score)
            })
            .max_by(|(_, a), (_, b)| a.total_cmp(b))
            .map(|(line, _)| line)
            .unwrap();

        let (inliers, unused_points) = self
            .unused_points
            .iter()
            .partition(|&&point| lines.distance_to_point(point) <= self.inlier_threshold);

        self.unused_points = unused_points;

        Some((lines, inliers))
    }
}
