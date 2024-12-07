use bevy::prelude::*;
use itertools::Itertools;
use nalgebra::Point2;

/// Inlier points of a line candidate
#[derive(Debug, Deref, DerefMut)]
pub struct Inliers(Vec<Point2<f32>>);

impl Inliers {
    pub fn new(inliers: Vec<Point2<f32>>) -> Self {
        let mut inliers = Self(inliers);
        inliers.sort_by_x();
        inliers
    }

    /// Extend the inliers with the inliers of another line candidate
    pub fn extend(&mut self, other: Self) {
        self.0.extend(other.0.into_iter());
    }

    /// Split the line candidate into multiple candidates, every time the gap between two neighboring inliers is too large
    ///
    /// Returns a vector of the separated line candidates
    pub fn split_at_gap(mut self, max_gap: f32) -> Vec<Self> {
        let mut candidates = vec![];

        while let Some(candidate) = self.split_at_gap_single(max_gap) {
            candidates.push(candidate);
        }
        candidates.push(self);

        // handle edge case where the first inlier is too far from the second
        candidates.retain(|c| c.0.len() >= 2);

        candidates
    }

    fn sort_by_x(&mut self) {
        self.0.sort_unstable_by(|a, b| a.x.total_cmp(&b.x));
    }

    /// Split the line candidate into two candidates at the first point where the gap between two neighboring inliers is too large
    ///
    /// If no such point is found, leaves the candidate unchanged and returns `None`
    ///
    /// If such a point is found, mutates the current candidate and returns the new candidate that was split off
    fn split_at_gap_single(&mut self, max_gap: f32) -> Option<Self> {
        let split_index = self
            .0
            .iter()
            // (i, inlier)
            .enumerate()
            .rev()
            // ((i, inlier), (i-1, prev_inlier))
            .tuple_windows::<(_, _)>()
            .find_map(|((i, inlier), (_, prev_inlier))| {
                // find the first point where the gap between two neighboring inliers is too large
                if nalgebra::distance(inlier, prev_inlier) > max_gap {
                    Some(i)
                } else {
                    None
                }
            })?;

        let new_inliers = self.0.split_off(split_index);

        Some(Self(new_inliers))
    }
}
