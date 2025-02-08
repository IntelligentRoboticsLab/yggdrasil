//! Geometric objects.

use std::f32::consts::{PI, TAU};

use nalgebra as na;

pub type Point = na::Point2<f32>;
pub type Vector = na::Vector2<f32>;
pub type Isometry = na::Isometry2<f32>;

/// The rotation direction around a circle.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum RotationDirection {
    /// Counterclockwise
    Ccw,
    /// Clockwise
    Cw,
}

impl RotationDirection {
    /// The angular distance from `start` to `end`.
    pub fn angular_distance(self, start: f32, end: f32) -> f32 {
        match self {
            Self::Ccw => (end - start).rem_euclid(TAU),
            Self::Cw => -(start - end).rem_euclid(TAU),
        }
    }

    /// Checks whether `closer` is closer than `further`.
    pub fn closer_or_equal(closer: f32, further: f32) -> bool {
        match further >= 0. {
            true => closer <= further,
            false => closer >= further,
        }
    }
}

/// A line segment from `start` to `end`.
#[derive(Copy, Clone, Debug)]
pub struct LineSegment {
    pub start: Point,
    pub end: Point,
}

impl LineSegment {
    /// Creates a new line segment from `start` to `end`.
    #[must_use]
    pub fn new(start: Point, end: Point) -> Self {
        Self { start, end }
    }

    /// Returns a vector in the direction of the line segment.
    #[must_use]
    pub fn direction(self) -> Vector {
        (self.end - self.start) / self.length()
    }

    /// Returns the angle the line segment points to.
    #[must_use]
    pub fn forward(self) -> f32 {
        let dir = self.direction();
        dir.y.atan2(dir.x)
    }

    /// Shortens the line segment to the projection of the point onto the segment.
    #[must_use]
    pub fn enter(self, point: Point) -> Option<Self> {
        let direction = self.direction();
        let length = self.length();

        let distance = (point - self.start).dot(&direction);

        if distance < 0. || distance > length {
            return None;
        }

        Some(Self::new(self.start + distance * direction, self.end))
    }
}

/// A circle defined by a `center` and a `radius`.
///
/// The behavior of a circle with a negative radius is unspecified.
#[derive(Copy, Clone, Debug)]
pub struct Circle {
    pub center: Point,
    pub radius: f32,
}

impl Circle {
    /// Creates a new circle.
    #[must_use]
    pub fn new(center: Point, radius: f32) -> Self {
        Self { center, radius }
    }

    /// Creates a new circle at the origin.
    #[must_use]
    pub fn origin(radius: f32) -> Self {
        Self::new(Point::origin(), radius)
    }

    /// Returns a copy of the circle with the dilation added to the radius.
    ///
    /// No checks are done to ensure the radius remains positive.
    #[must_use]
    pub fn dilate(self, dilation: f32) -> Self {
        Self {
            center: self.center,
            radius: self.radius + dilation,
        }
    }

    /// Returns the point on the circumference of the circle at a given angle.
    #[must_use]
    pub fn point_at_angle(self, angle: f32) -> Point {
        self.point_from_vector(Vector::new(angle.cos(), angle.sin()))
    }

    /// Projects a vector onto the circle such that a unit vector results in a point on the
    /// circumference.
    #[must_use]
    pub fn point_from_vector(self, direction: Vector) -> Point {
        self.center + self.radius * direction
    }

    /// Returns the angle from the circle's center to the point.
    #[must_use]
    pub fn angle_to_point(self, point: Point) -> f32 {
        let center_to_point = point - self.center;
        center_to_point.y.atan2(center_to_point.x)
    }

    /// Calculates the tangents from the circle to a point.
    ///
    /// Returns `None` if the tangents don't exist (i.e., the point is inside the circle).
    #[must_use]
    pub fn tangents(self, point: Point) -> Option<Tangents> {
        let center_to_point = point - self.center;
        let dist = center_to_point.norm();

        if dist <= self.radius {
            return None;
        }

        let angle_to_point = center_to_point.y.atan2(center_to_point.x);
        let angle_to_tangent = (self.radius / dist).acos();

        Some(Tangents {
            ccw: angle_to_point + angle_to_tangent,
            cw: angle_to_point - angle_to_tangent,
        })
    }

    /// Calculates the outer tangents from this circle to another.
    ///
    /// Returns `None` if the tangents don't exist (i.e., one of the circles is completely
    /// contained inside the other).
    pub fn outer_tangents(self, other: Circle) -> Option<Tangents> {
        if self.radius <= other.radius {
            self.outer_tangents_ordered(other)
        } else {
            other.outer_tangents_ordered(self).map(Tangents::flip)
        }
    }

    /// Calculates the outer tangents from a smaller to a larger circle.
    ///
    /// The behavior of this function is unspecified if `larger` is not actually larger.
    fn outer_tangents_ordered(self, larger: Circle) -> Option<Tangents> {
        larger.dilate(-self.radius).tangents(self.center)
    }

    /// Calculates the inner tangents from one circle to another.
    ///
    /// Returns `None` if the tangents don't exist (i.e., the circles partially or completely
    /// overlap).
    pub fn inner_tangents(self, other: Circle) -> Option<InnerTangents> {
        if self.radius <= other.radius {
            self.inner_tangents_ordered(other)
        } else {
            other.inner_tangents_ordered(self).map(InnerTangents::flip)
        }
    }

    /// Calculates the inner tangents from a smaller to a larger circle.
    ///
    /// The behavior of this function is unspecified if `larger` is not actually larger.
    fn inner_tangents_ordered(self, larger: Circle) -> Option<InnerTangents> {
        larger
            .dilate(self.radius)
            .tangents(self.center)
            .map(|t| InnerTangents {
                cw_to_ccw: (t.ccw + PI, t.ccw),
                ccw_to_cw: (t.cw - PI, t.cw),
            })
    }
}

/// The tangents from a circle to a point or other circle.
pub struct Tangents {
    pub ccw: f32,
    pub cw: f32,
}

impl Tangents {
    /// Gets the tangents associated with the given direction.
    #[must_use]
    pub fn get(self, direction: RotationDirection) -> f32 {
        match direction {
            RotationDirection::Ccw => self.ccw,
            RotationDirection::Cw => self.cw,
        }
    }

    /// Returns the counterclockwise angles as a tuple.
    #[must_use]
    pub fn ccw_to_ccw(self) -> (f32, f32) {
        (self.ccw, self.ccw)
    }

    /// Returns the clockwise angles as a tuple.
    #[must_use]
    pub fn cw_to_cw(self) -> (f32, f32) {
        (self.cw, self.cw)
    }

    /// Flips the direction of the tangents.
    #[must_use]
    pub fn flip(self) -> Self {
        Self {
            ccw: self.cw,
            cw: self.ccw,
        }
    }
}

/// The inner (transverse) tangents of two circles.
pub struct InnerTangents {
    pub ccw_to_cw: (f32, f32),
    pub cw_to_ccw: (f32, f32),
}

impl InnerTangents {
    /// Gets the tangents associated with the given start direction.
    #[must_use]
    pub fn get(self, direction: RotationDirection) -> (f32, f32) {
        match direction {
            RotationDirection::Ccw => self.ccw_to_cw,
            RotationDirection::Cw => self.cw_to_ccw,
        }
    }

    /// Flips the direction of the tangents.
    #[must_use]
    pub fn flip(self) -> Self {
        Self {
            ccw_to_cw: (self.ccw_to_cw.1, self.ccw_to_cw.0),
            cw_to_ccw: (self.cw_to_ccw.1, self.cw_to_ccw.0),
        }
    }
}

/// A circular arc with a direction, defined by a `circle`, a `start` angle, and a `step` such that
/// the end angle is defined as `start + step`.
#[derive(Copy, Clone, Debug)]
pub struct CircularArc {
    pub circle: Circle,
    pub start: f32,
    pub step: f32,
}

impl CircularArc {
    /// Creates a new arc.
    #[must_use]
    pub fn new(
        circle: Circle,
        direction: RotationDirection,
        start: f32,
        end: f32,
    ) -> Self {
        Self {
            circle,
            start,
            step: direction.angular_distance(start, end),
        }
    }

    /// Creates an arc through an isometry with the given direction and radius.
    #[must_use]
    pub fn from_isometry(
        isometry: Isometry,
        direction: RotationDirection,
        radius: f32,
    ) -> Self {
        match direction {
            RotationDirection::Ccw => Self::ccw_from_isometry(isometry, radius),
            RotationDirection::Cw => Self::cw_from_isometry(isometry, radius),
        }
    }

    /// Creates a counterclockwise arc through an isometry with the given radius.
    #[must_use]
    fn ccw_from_isometry(isometry: Isometry, radius: f32) -> Self {
        Self {
            circle: Circle::new(isometry * na::point![0., radius], radius),
            start: isometry.rotation.angle() - 0.5 * PI,
            step: TAU,
        }
    }

    /// Creates a clockwise arc through an isometry with the given radius.
    #[must_use]
    fn cw_from_isometry(isometry: Isometry, radius: f32) -> Self {
        Self {
            circle: Circle::new(isometry * na::point![0., -radius], radius),
            start: isometry.rotation.angle() + 0.5 * PI,
            step: -TAU,
        }
    }

    /// Returns a copy of this arc with a different start.
    #[must_use]
    pub fn with_start(mut self, start: f32) -> Self {
        self.start = start;
        self
    }

    /// Returns a copy of this arc with a different step.
    #[must_use]
    pub fn with_step(mut self, step: f32) -> Self {
        self.step = step;
        self
    }

    /// Changes the direction of the arc.
    #[must_use]
    pub fn with_direction(self, direction: RotationDirection) -> Self {
        if self.direction() != direction {
            self.flip()
        } else {
            self
        }
    }

    /// Flips the direction of the arc.
    #[must_use]
    pub fn flip(self) -> Self {
        Self {
            circle: self.circle,
            start: self.end(),
            step: -self.step,
        }
    }

    /// Returns the direction of the arc.
    #[must_use]
    pub fn direction(self) -> RotationDirection {
        match self.step >= 0. {
            true => RotationDirection::Ccw,
            false => RotationDirection::Cw,
        }
    }

    /// The angular distance from `start` to `end`.
    pub fn angular_distance(self, start: f32, end: f32) -> f32 {
        self.direction().angular_distance(start, end)
    }

    /// Returns whether the arc is a full circle (i.e, the absolute step is or exceeds `TAU`).
    #[must_use]
    pub fn full_circle(self) -> bool {
        self.step.abs() >= TAU
    }


    /// Returns the angle at the end of the arc.
    #[must_use]
    pub fn end(self) -> f32 {
        self.start + self.step
    }

    /// Returns the turn such that a left (i.e., counterclockwise) turn is positive.
    #[must_use]
    pub fn turn(self) -> f32 {
        self.step.signum() / self.circle.radius
    }

    /// Returns the angle pointing forward from the start.
    #[must_use]
    pub fn forward_at_start(self) -> f32 {
        self.forward_at_angle(self.start)
    }

    /// Returns the angle pointing forward from the end.
    #[must_use]
    pub fn forward_at_end(self) -> f32 {
        self.forward_at_angle(self.end())
    }

    /// Returns the angle pointing forward at an angle on the arc.
    #[must_use]
    pub fn forward_at_angle(self, angle: f32) -> f32 {
        angle + self.step.signum() * 0.5 * PI
    }

    /// Returns whether the angle is contained within this arc.
    #[must_use]
    pub fn contains_angle(self, angle: f32) -> bool {
        let distance = self.angular_distance(self.start, angle);
        RotationDirection::closer_or_equal(distance, self.step)
    }

    /// Returns the point at an angle.
    #[must_use]
    pub fn point_at_angle(self, angle: f32) -> Point {
        self.circle.point_at_angle(angle)
    }

    /// Returns the point at the start.
    #[must_use]
    pub fn point_at_start(self) -> Point {
        self.point_at_angle(self.start)
    }

    /// Returns the point at the end.
    #[must_use]
    pub fn point_at_end(self) -> Point {
        self.point_at_angle(self.end())
    }

    /// Same as `enter_non_circular`, but preserves circles.
    #[must_use]
    pub fn enter(self, start: f32) -> Option<Self> {
        if self.full_circle() {
            Some(self.with_start(start))
        } else {
            self.enter_non_circular(start)
        }
    }

    /// Returns a new arc that lies within this arc with the given start.
    ///
    /// Circles are not preserved.
    #[must_use]
    pub fn enter_non_circular(self, start: f32) -> Option<Self> {
        let step = self.angular_distance(start, self.end());

        RotationDirection::closer_or_equal(step, self.step)
            .then(|| self.with_start(start).with_step(step))
    }

    /// Returns a new shortened copy of this arc with a given end if that end lies on this arc.
    #[must_use]
    pub fn exit(self, end: f32) -> Option<Self> {
        let step = self.angular_distance(self.start, end);

        RotationDirection::closer_or_equal(step, self.step).then(|| self.with_step(step))
    }

    /// Returns a copy of this arc such that it starts at the tangent line through `point`.
    #[must_use]
    pub fn point_to_arc(mut self, point: Point) -> Option<(LineSegment, Self)> {
        self = self.enter(self.circle.tangents(point)?.get(self.direction()))?;

        Some((LineSegment::new(point, self.point_at_start()), self))
    }

    /// Returns a copy of this arc such that it ends at the tangent line through `point`.
    #[must_use]
    pub fn arc_to_point(mut self, point: Point) -> Option<(Self, LineSegment)> {
        self = self.enter(self.circle.tangents(point)?.flip().get(self.direction()))?;

        Some((self, LineSegment::new(self.point_at_end(), point)))
    }

    /// Connects two arcs together by their common tangent.
    #[must_use]
    pub fn arc_to_arc(mut self, mut other: Self) -> Option<(Self, LineSegment, Self)> {
        use RotationDirection::{Ccw, Cw};

        let angles = match (self.direction(), other.direction()) {
            (Ccw, Ccw) => self.circle.outer_tangents(other.circle)?.ccw_to_ccw(),
            (Ccw, Cw) => self.circle.inner_tangents(other.circle)?.ccw_to_cw,
            (Cw, Ccw) => self.circle.inner_tangents(other.circle)?.cw_to_ccw,
            (Cw, Cw) => self.circle.outer_tangents(other.circle)?.cw_to_cw(),
        };

        self = self.exit(angles.0)?;
        other = other.enter(angles.1)?;

        Some((
            self,
            LineSegment::new(self.point_at_end(), other.point_at_start()),
            other,
        ))
    }

    /// Returns an iterator of vertices on the arc such that a full circle has `resolution`
    /// vertices.
    pub fn vertices(self, resolution: f32) -> impl Iterator<Item = Point> {
        self.n_vertices(((resolution * self.step.abs() / TAU).ceil() as usize).max(2))
    }

    /// Returns an iterator of vertices on the arc such that there are `n` equally spaced vertices.
    pub fn n_vertices(self, n: usize) -> impl Iterator<Item = Point> {
        let factor = self.step * ((n - 1) as f32).recip();

        (0..n).map(move |i| self.circle.point_at_angle(self.start + i as f32 * factor))
    }
}

impl From<Circle> for CircularArc {
    fn from(circle: Circle) -> Self {
        Self {
            circle,
            start: 0.,
            step: TAU,
        }
    }
}

/// Geometric objects that have a length.
pub trait Length {
    /// Returns the geometric length of the object.
    fn length(self) -> f32;
}

impl Length for LineSegment {
    fn length(self) -> f32 {
        na::distance(&self.start, &self.end)
    }
}

impl Length for CircularArc {
    fn length(self) -> f32 {
        self.step.abs() * self.circle.radius
    }
}

// TODO: the intersection code is messy, and we don't actually need the intersection points.

/// Trait for determining if and where two geometric objects intersect.
pub trait Intersects<T> {
    type Intersection;

    /// Tests if and where two geometric objects intersect.
    fn intersects(self, other: T) -> Self::Intersection;
}

impl Intersects<LineSegment> for LineSegment {
    type Intersection = bool;

    fn intersects(self, other: LineSegment) -> Self::Intersection {
        // evil but i can't be bothered to come up with better names
        // + i'll probably rewrite the intersection interface and implementation
        #![allow(clippy::many_single_char_names)]

        let u = self.end - self.start;
        let v = other.end - other.start;
        let w = other.start - self.start;

        let s = v.y * w.x - u.y * w.y;
        let t = v.x * w.x - u.x * w.y;
        let range = 0. ..=v.y * u.x - v.x * u.y;

        range.contains(&s) && range.contains(&t)
    }
}

impl Intersects<LineSegment> for CircularArc {
    type Intersection = (Option<f32>, Option<f32>);

    fn intersects(self, other: LineSegment) -> Self::Intersection {
        // TODO: properly handle this
        let mut circle = self.circle;
        circle.radius = (0.99 * circle.radius).max(0.);

        let (entry, exit) = circle.intersects(other);
        let contains = |angle: &f32| self.contains_angle(*angle);

        (entry.filter(contains), exit.filter(contains))
    }
}

impl Intersects<CircularArc> for CircularArc {
    type Intersection = (Option<f32>, Option<f32>);

    fn intersects(self, other: CircularArc) -> Self::Intersection {
        let Some(Tangents { ccw, cw }) = self.circle.intersects(other.circle) else {
            return (None, None);
        };

        (
            self.contains_angle(ccw)
                .then(|| {
                    let angle = other.circle.angle_to_point(self.point_at_angle(ccw));
                    other.contains_angle(angle).then_some(ccw)
                })
                .flatten(),
            self.contains_angle(cw)
                .then(|| {
                    let angle = other.circle.angle_to_point(self.point_at_angle(cw));
                    other.contains_angle(angle).then_some(cw)
                })
                .flatten(),
        )
    }
}

impl Intersects<LineSegment> for Circle {
    type Intersection = (Option<f32>, Option<f32>);

    fn intersects(self, other: LineSegment) -> Self::Intersection {
        let start_to_end = other.end - other.start;
        let start_to_center = self.center - other.start;

        let length = start_to_end.norm();
        let parallel = start_to_end / length;
        let perpendicular = Vector::new(parallel.y, -parallel.x);

        let distance = parallel.dot(&start_to_center);

        let adjacent = perpendicular.dot(&start_to_center);
        let opposite = (self.radius * self.radius - adjacent * adjacent).sqrt();

        if opposite.is_nan() {
            return (None, None);
        }

        let (angle_to_line, angle_to_intersection) = if adjacent >= 0. {
            (
                f32::atan2(-perpendicular.y, -perpendicular.x),
                (opposite / self.radius).asin(),
            )
        } else {
            (
                f32::atan2(perpendicular.y, perpendicular.x),
                -(opposite / self.radius).asin(),
            )
        };

        let range = 0.0..=length;
        (
            range
                .contains(&(distance - opposite))
                .then_some(angle_to_line + angle_to_intersection),
            range
                .contains(&(distance + opposite))
                .then_some(angle_to_line - angle_to_intersection),
        )
    }
}

impl Intersects<Circle> for Circle {
    type Intersection = Option<Tangents>;

    fn intersects(self, other: Circle) -> Self::Intersection {
        let center_to_center = other.center - self.center;
        let a2 = center_to_center.norm_squared();

        if a2 == 0. {
            return None;
        }

        let b2 = self.radius * self.radius;
        let c2 = other.radius * other.radius;

        let cos = (a2 + b2 - c2) / (2. * a2.sqrt() * self.radius);

        if !(-1.0..=1.0).contains(&cos) {
            return None;
        }

        let angle_to_center = center_to_center.y.atan2(center_to_center.x);
        let angle_to_intersection = cos.acos();

        Some(Tangents {
            ccw: angle_to_center + angle_to_intersection,
            cw: angle_to_center - angle_to_intersection,
        })
    }
}

/// Segment of a path, can be either a straight line segment or a circular arc.
#[derive(Copy, Clone, Debug)]
pub enum Segment {
    LineSegment(LineSegment),
    CircularArc(CircularArc),
}

impl Segment {
    /// Returns the start of this segment.
    #[must_use]
    pub fn start(self) -> Point {
        match self {
            Segment::LineSegment(line) => line.start,
            Segment::CircularArc(arc) => arc.point_at_start(),
        }
    }

    /// Returns the end of this segment.
    #[must_use]
    pub fn end(self) -> Point {
        match self {
            Segment::LineSegment(line) => line.end,
            Segment::CircularArc(arc) => arc.point_at_end(),
        }
    }

    /// Returns the turn such that a left (i.e., counterclockwise) turn is positive.
    #[must_use]
    pub fn turn(self) -> f32 {
        match self {
            Segment::LineSegment(_) => 0.,
            Segment::CircularArc(arc) => arc.turn(),
        }
    }

    /// Returns the forward angle of this segment.
    #[must_use]
    pub fn forward_at_start(self) -> f32 {
        match self {
            Segment::LineSegment(line) => line.forward(),
            Segment::CircularArc(arc) => arc.forward_at_start(),
        }
    }

    /// Returns the forward angle of this segment.
    #[must_use]
    pub fn forward_at_end(self) -> f32 {
        match self {
            Segment::LineSegment(line) => line.forward(),
            Segment::CircularArc(arc) => arc.forward_at_end(),
        }
    }

    /// Shortens this segment to the poiht closest to the given point.
    pub fn shorten(&mut self, point: Point) {
        match self {
            Segment::LineSegment(line) => {
                if let Some(new) = line.enter(point) {
                    *line = new;
                }
            }
            Segment::CircularArc(arc) => {
                if let Some(new) = arc.enter(arc.circle.angle_to_point(point)) {
                    *arc = new;
                }
            }
        }
    }

    /// Returns the vertices to render this segment.
    #[must_use]
    pub fn vertices(self, resolution: f32) -> Vec<Point> {
        match self {
            Segment::LineSegment(line) => vec![line.start, line.end],
            Segment::CircularArc(arc) => arc.vertices(resolution).collect(),
        }
    }
}

impl From<LineSegment> for Segment {
    fn from(line: LineSegment) -> Self {
        Self::LineSegment(line)
    }
}

impl From<CircularArc> for Segment {
    fn from(arc: CircularArc) -> Self {
        Self::CircularArc(arc)
    }
}

impl Intersects<Segment> for Segment {
    type Intersection = bool;

    fn intersects(self, other: Segment) -> Self::Intersection {
        match (self, other) {
            (Segment::LineSegment(a), Segment::LineSegment(b)) => a.intersects(b),
            (Segment::LineSegment(a), Segment::CircularArc(b)) => {
                let (enter, exit) = b.intersects(a);
                enter.is_some() || exit.is_some()
            }
            (Segment::CircularArc(a), Segment::LineSegment(b)) => {
                let (enter, exit) = a.intersects(b);
                enter.is_some() || exit.is_some()
            }
            (Segment::CircularArc(a), Segment::CircularArc(b)) => {
                let (a, b) = a.intersects(b);
                a.is_some() || b.is_some()
            }
        }
    }
}
