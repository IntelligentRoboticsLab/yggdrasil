use nalgebra::{Isometry2, Point2, Vector2};

use crate::{
    core::config::layout::{FieldLine, LayoutConfig},
    vision::line_detection::{
        line::{Circle, LineSegment2},
        DetectedLines,
    },
};

/// Correspondence between a measured line and a field line
#[derive(Clone, Debug)]
pub struct FieldLineCorrespondence {
    /// Measured line in field space
    pub measurement: LineSegment2,
    /// Corresponding reference line in field space
    pub reference: FieldLine,
    /// Start point of the correspondence
    pub start: PointCorrespondence,
    /// End point of the correspondence
    pub end: PointCorrespondence,
}

/// Factor by which the length of a measured line may be greater than the corresponding field line
const LINE_LENGTH_ACCEPTANCE_FACTOR: f32 = 1.5;

/// Matches detected lines to their closest field lines.
#[must_use]
pub fn correspond_field_lines(
    lines: &DetectedLines,
    layout: &LayoutConfig,
    correction: Isometry2<f32>,
) -> Vec<FieldLineCorrespondence> {
    lines
        .segments
        .iter()
        .filter_map(|&measurement| {
            let (_weight, correspondences, corrected_measurement, reference) = layout
                .field
                .field_lines()
                .iter()
                .filter_map(|&reference| {
                    let corrected_measurement = correction * measurement;

                    let measurement_length = measurement.length();
                    let reference_length = match reference {
                        FieldLine::Segment(segment) => segment.length(),
                        // approximate length of a detected line in the center circle
                        FieldLine::Circle(circle) => circle.radius,
                    };

                    if measurement_length < reference_length * LINE_LENGTH_ACCEPTANCE_FACTOR {
                        return None;
                    }

                    let correspondences = correspond_lines(reference, measurement);

                    let angle_weight = correspondences
                        .measurement_direction
                        .dot(&correspondences.reference_direction)
                        .abs();
                    let length_weight = measurement_length / reference_length;

                    let weight = 1.0 / (angle_weight + length_weight);

                    if weight > 0.0 {
                        Some((weight, correspondences, corrected_measurement, reference))
                    } else {
                        None
                    }
                })
                .min_by(|(w1, c1, _, _), (w2, c2, _, _)| {
                    let weighted_distance_1 = w1 * (c1.start.distance() + c1.end.distance());
                    let weighted_distance_2 = w2 * (c2.start.distance() + c2.end.distance());
                    weighted_distance_1.total_cmp(&weighted_distance_2)
                })?;

            let inverse_correction = correction.inverse();

            Some(FieldLineCorrespondence {
                measurement: inverse_correction * corrected_measurement,
                reference,
                start: PointCorrespondence {
                    measurement: inverse_correction * correspondences.start.measurement,
                    reference: correspondences.start.reference,
                },
                end: PointCorrespondence {
                    measurement: inverse_correction * correspondences.end.measurement,
                    reference: correspondences.end.reference,
                },
            })
        })
        .collect()
}

/// Correspondence between two points
#[derive(Clone, Copy, Debug)]
pub struct PointCorrespondence {
    pub measurement: Point2<f32>,
    pub reference: Point2<f32>,
}

impl PointCorrespondence {
    /// Distance between measurement and reference points in meters
    #[must_use]
    pub fn distance(&self) -> f32 {
        nalgebra::distance(&self.measurement, &self.reference)
    }
}

/// Correspondence between two line segments
#[derive(Clone, Debug)]
pub struct LineCorrespondence {
    pub measurement_direction: Vector2<f32>,
    pub reference_direction: Vector2<f32>,
    /// Start point correspondence
    pub start: PointCorrespondence,
    /// End point correspondence
    pub end: PointCorrespondence,
}

#[must_use]
fn correspond_lines(reference: FieldLine, measurement: LineSegment2) -> LineCorrespondence {
    match reference {
        FieldLine::Segment(reference) => correspond_segment(reference, measurement),
        FieldLine::Circle(reference) => correspond_circle(reference, measurement),
    }
}

fn correspond_segment(reference: LineSegment2, measurement: LineSegment2) -> LineCorrespondence {
    let measurement = match [
        nalgebra::distance(&measurement.start, &reference.start),
        nalgebra::distance(&measurement.end, &reference.end),
        nalgebra::distance(&measurement.start, &reference.end),
        nalgebra::distance(&measurement.end, &reference.start),
    ]
    .iter()
    .enumerate()
    .min_by(|(_, a), (_, b)| a.total_cmp(b))
    .unwrap()
    .0
    {
        2 | 3 => measurement.to_flipped(),
        _ => measurement,
    };

    let measurement_direction = (measurement.start - measurement.end).normalize();
    let reference_direction = (reference.start - reference.end).normalize();

    let (projected_point_on_measurement, measured_distance) =
        measurement.project_with_distance(reference.start);

    let (projected_point_on_reference, reference_distance) =
        reference.project_with_distance(measurement.start);

    let correspondence_start = if measured_distance < reference_distance {
        PointCorrespondence {
            measurement: projected_point_on_measurement,
            reference: reference.start,
        }
    } else {
        PointCorrespondence {
            measurement: measurement.start,
            reference: projected_point_on_reference,
        }
    };

    let (projected_point_on_measurement, measured_distance) =
        measurement.project_with_distance(reference.end);
    let (projected_point_on_reference, reference_distance) =
        reference.project_with_distance(measurement.end);

    let correspondence_end = if measured_distance < reference_distance {
        PointCorrespondence {
            measurement: projected_point_on_measurement,
            reference: reference.end,
        }
    } else {
        PointCorrespondence {
            measurement: measurement.end,
            reference: projected_point_on_reference,
        }
    };

    LineCorrespondence {
        measurement_direction,
        reference_direction,
        start: correspondence_start,
        end: correspondence_end,
    }
}

fn correspond_circle(reference: Circle, measurement: LineSegment2) -> LineCorrespondence {
    let center_to_start = measurement.start - reference.center;
    let center_to_end = measurement.end - reference.center;

    let reference_start = if let Some(norm) = center_to_start.try_normalize(f32::EPSILON) {
        reference.center + norm * reference.radius
    } else {
        Point2::new(reference.center.x + reference.radius, reference.center.y)
    };

    let reference_end = if let Some(norm) = center_to_end.try_normalize(f32::EPSILON) {
        reference.center + norm * reference.radius
    } else {
        Point2::new(reference.center.x + reference.radius, reference.center.y)
    };

    let correspondence_start = PointCorrespondence {
        measurement: measurement.start,
        reference: reference_start,
    };
    let correspondence_end = PointCorrespondence {
        measurement: measurement.end,
        reference: reference_end,
    };

    let measurement_direction = (measurement.start - measurement.end).normalize();
    let center_vector = (reference_start - reference.center) + (reference_end - reference.center);
    // rotate the center vector 90 degrees counterclockwise
    let reference_direction = Vector2::new(-center_vector.y, center_vector.x).normalize();

    LineCorrespondence {
        measurement_direction,
        reference_direction,
        start: correspondence_start,
        end: correspondence_end,
    }
}
