pub mod line;
// use nalgebra::Point2;
// use rand::{seq::SliceRandom, Rng};

// use super::line::Line2;

// #[derive(Debug, PartialEq)]
// pub struct RansacResult {
//     pub line: Line2,
//     pub inliers: Vec<Point2<f32>>,
// }

// pub struct Ransac {
//     pub unused_points: Vec<Point2<f32>>,
// }

// impl Ransac {
//     pub fn new(unused_points: Vec<Point2<f32>>) -> Ransac {
//         Ransac { unused_points }
//     }
// }

// impl Ransac {
//     pub fn next_line<R: Rng>(
//         &mut self,
//         rng: &mut R,
//         iterations: usize,
//         maximum_score_distance: f32,
//         maximum_inclusion_distance: f32,
//     ) -> Option<RansacResult> {
//         if self.unused_points.len() < 2 {
//             return None;
//         }

//         let maximum_score_distance_squared = maximum_score_distance * maximum_score_distance;
//         let maximum_inclusion_distance_squared =
//             maximum_inclusion_distance * maximum_inclusion_distance;
//         let best_line = (0..iterations)
//             .map(|_| {
//                 let mut points = self.unused_points.choose_multiple(rng, 2);
//                 let line = Line2::from_points(*points.next().unwrap(), *points.next().unwrap());
//                 let score: f32 = self
//                     .unused_points
//                     .iter()
//                     .filter(|&point| {
//                         line.squared_distance_to(*point) <= maximum_score_distance_squared
//                     })
//                     .map(|point| 1.0 - line.distance_to(*point) / maximum_score_distance)
//                     .sum();
//                 (line, score)
//             })
//             .max_by(|(_, score_a), (_, score_b)| score_a.total_cmp(score_b))
//             .expect("max_by_key erroneously returned no result")
//             .0;
//         let (used_points, unused_points) = self.unused_points.iter().partition(|point| {
//             best_line.squared_distance_to(**point) <= maximum_inclusion_distance_squared
//         });
//         self.unused_points = unused_points;
//         Some(RansacResult {
//             line: best_line,
//             inliers: used_points,
//         })
//     }
// }

pub trait Ransac: Sized {
    const MIN_SAMPLES: usize;

    type Model;
    type Data;

    fn next(&mut self) -> Option<(Self::Model, Vec<Self::Data>)>;

    fn residual(&self, model: &Self::Model, point: &Self::Data) -> f32;
    fn score(&self, model: &Self::Model, point: &Self::Data) -> f32;
}
