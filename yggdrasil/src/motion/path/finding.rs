//! Lower-level pathfinding capabilities.

use nalgebra as na;
use ordered_float::OrderedFloat;

use super::obstacles::Colliders;
use super::{
    geometry::{CircularArc, Intersects, Isometry, Length, LineSegment, Point, Segment},
    PathSettings,
};

type Cost = OrderedFloat<f32>;

/// Struct containing all the data necessary for pathfinding.
#[derive(Copy, Clone)]
pub struct Pathfinding<'a> {
    /// The start position of the path.
    pub start: Position,
    /// The goal position of the path.
    pub goal: Position,
    /// The colliders to navigate around.
    pub colliders: &'a Colliders,
    /// The settings for pathfinding.
    pub settings: &'a PathSettings,
}

impl Pathfinding<'_> {
    /// Calculates the shortest path from `start` to `goal` and returns the segments that make up
    /// the path as well as the total length (if such a path exists).
    #[must_use]
    pub fn path(&self) -> Option<(Vec<Segment>, f32)> {
        let (states, cost) = self.astar()?;
        let mut segments = Vec::new();

        for window in states.windows(2) {
            match *window {
                // This sequence can only occur when both the start and goal are points.
                [State::Start, State::Goal] => {
                    let prev = self.start.point().unwrap();
                    let next = self.goal.point().unwrap();

                    segments.push(LineSegment::new(prev, next).into());
                }
                // Special case when the start is a point (but the goal is not).
                [State::Start, next] => {
                    if let Some(start) = self.start.point() {
                        let next = self.arc(next).unwrap();
                        let (a, _) = next.point_to_arc(start).unwrap();

                        segments.push(a.into());
                    }
                }
                // If this is the last state, finish the path.
                [prev, State::Goal] => match self.goal {
                    Position::Isometry(_) => {
                        let prev = self.arc(prev).unwrap();

                        segments.push(prev.into());
                    }
                    Position::Point(goal) => {
                        let prev = self.arc(prev).unwrap();
                        let (a, b) = prev.arc_to_point(goal).unwrap();

                        segments.push(a.into());
                        segments.push(b.into());
                    }
                },
                // Otherwise, connect the two states.
                [prev, next] => {
                    let prev = self.arc(prev).unwrap();
                    let next = self.arc(next).unwrap();
                    let (a, b, _) = prev.arc_to_arc(next).unwrap();

                    segments.push(a.into());
                    segments.push(b.into());
                }
                // Since we use `states.windows(2)`, this is unreachable.
                _ => unreachable!(),
            }
        }

        Some((segments, cost.into()))
    }

    /// `FInds` the shortest path using the A* algorithm.
    fn astar(&self) -> Option<(Vec<State>, Cost)> {
        pathfinding::directed::astar::astar(
            &State::Start,
            |state| self.successors(*state),
            |state| self.heuristic(*state),
            |state| matches!(state, State::Goal),
        )
    }

    /// Returns the successors of a given state.
    fn successors(&self, state: State) -> Vec<(State, Cost)> {
        match state {
            State::Start => self.start_successors(),
            State::Goal => unreachable!(),
            State::CcwEaseOut(_) | State::CwEaseOut(_) => {
                let arc = self.arc(state).unwrap();

                if self.collides(arc) {
                    Vec::new()
                } else {
                    vec![(State::Goal, arc.length().into())]
                }
            }
            state => self.arc_successors(self.arc(state).unwrap()),
        }
    }

    /// Returns the heuristic for A* based on the euclidean distance to the goal.
    fn heuristic(&self, state: State) -> Cost {
        na::distance(&self.point(state), &self.goal.to_point()).into()
    }

    /// Returns the successors from [`State::Start`].
    fn start_successors(&self) -> Vec<(State, Cost)> {
        match self.start {
            Position::Isometry(_) => vec![
                (State::CcwEaseIn, 0.0.into()),
                (State::CwEaseIn, 0.0.into()),
            ],
            Position::Point(start) => self.point_successors(start),
        }
    }

    /// Returns the successors from [`State::Start`] if the start is a [`Position::Point`].
    fn point_successors(&self, point: Point) -> Vec<(State, Cost)> {
        // Potential successors that directly lead to the goal.
        let direct = [
            self.goal
                .point()
                .map(|goal| LineSegment::new(point, goal))
                .filter(|line| !self.collides(*line))
                .map(|line| (State::Goal, line.length().into())),
            self.ccw_ease_out()
                .and_then(|ease_out| ease_out.point_to_arc(point))
                .filter(|(a, _)| !self.collides(*a))
                .map(|(a, b)| (State::CcwEaseOut(b.start.into()), a.length().into())),
            self.cw_ease_out()
                .and_then(|ease_out| ease_out.point_to_arc(point))
                .filter(|(a, _)| !self.collides(*a))
                .map(|(a, b)| (State::CwEaseOut(b.start.into()), a.length().into())),
        ]
        .into_iter()
        .flatten();

        // Potential successors that don't directly lead to the goal.
        let indirect = self
            .colliders
            .arcs
            .iter()
            .enumerate()
            .flat_map(|(index, other)| {
                [
                    other
                        .to_ccw()
                        .point_to_arc(point)
                        .filter(|(a, _)| !self.collides(*a))
                        .map(|(a, b)| {
                            (
                                State::CcwAlongCollider(index, b.start.into()),
                                a.length().into(),
                            )
                        }),
                    other
                        .to_cw()
                        .point_to_arc(point)
                        .filter(|(a, _)| !self.collides(*a))
                        .map(|(a, b)| {
                            (
                                State::CwAlongCollider(index, b.start.into()),
                                a.length().into(),
                            )
                        }),
                ]
                .into_iter()
                .flatten()
            });

        direct.chain(indirect).collect()
    }

    /// Returns the successors from a state that represents an arc.
    fn arc_successors(&self, arc: CircularArc) -> Vec<(State, Cost)> {
        // Potential successors that directly lead to the goal.
        let direct = [
            self.goal
                .point()
                .and_then(|goal| arc.arc_to_point(goal))
                .filter(|(a, b)| !self.collides(*a) && !self.collides(*b))
                .map(|(a, b)| (State::Goal, (a.length() + b.length()).into())),
            self.ccw_ease_out()
                .and_then(|ease_out| arc.arc_to_arc(ease_out))
                .filter(|(a, b, _)| !self.collides(*a) && !self.collides(*b))
                .map(|(a, b, c)| {
                    (
                        State::CcwEaseOut(c.start.into()),
                        (a.length() + b.length()).into(),
                    )
                }),
            self.cw_ease_out()
                .and_then(|ease_out| arc.arc_to_arc(ease_out))
                .filter(|(a, b, _)| !self.collides(*a) && !self.collides(*b))
                .map(|(a, b, c)| {
                    (
                        State::CwEaseOut(c.start.into()),
                        (a.length() + b.length()).into(),
                    )
                }),
        ]
        .into_iter()
        .flatten();

        // Potential successors that don't directly lead to the goal.
        let indirect = self
            .colliders
            .arcs
            .iter()
            .enumerate()
            .flat_map(|(index, other)| {
                [
                    arc.arc_to_arc(other.to_ccw())
                        .filter(|(a, b, _)| !self.collides(*a) && !self.collides(*b))
                        .map(|(a, b, c)| {
                            (
                                State::CcwAlongCollider(index, c.start.into()),
                                (a.length() + b.length()).into(),
                            )
                        }),
                    arc.arc_to_arc(other.to_cw())
                        .filter(|(a, b, _)| !self.collides(*a) && !self.collides(*b))
                        .map(|(a, b, c)| {
                            (
                                State::CwAlongCollider(index, c.start.into()),
                                (a.length() + b.length()).into(),
                            )
                        }),
                ]
                .into_iter()
                .flatten()
            });

        direct.chain(indirect).collect()
    }

    /// Get the `Point` this [`State`] represents.
    fn point(&self, state: State) -> Point {
        match state {
            State::Start => self.start.to_point(),
            State::Goal => self.goal.to_point(),
            state => self.arc(state).unwrap().point_at_start(),
        }
    }

    /// Get the `CircularArc` this [`State`] represents.
    ///
    /// Returns `None` if the state does not represent an arc (i.e., [`State::Start`] and
    /// [`State::Goal`].
    fn arc(&self, state: State) -> Option<CircularArc> {
        match state {
            State::Start => None,
            State::Goal => None,
            State::CcwEaseIn => self.ccw_ease_in(),
            State::CwEaseIn => self.cw_ease_in(),
            State::CcwAlongCollider(index, start) => {
                self.colliders.arcs[index].to_ccw().enter(start.into())
            }
            State::CwAlongCollider(index, start) => {
                self.colliders.arcs[index].to_cw().enter(start.into())
            }
            State::CcwEaseOut(start) => self.ccw_ease_out()?.enter_non_circular(start.into()),
            State::CwEaseOut(start) => self.cw_ease_out()?.enter_non_circular(start.into()),
        }
    }

    /// Returns the arc to ease in from the start counterclockwise.
    fn ccw_ease_in(&self) -> Option<CircularArc> {
        Some(CircularArc::ccw_from_isometry(
            self.start.isometry()?,
            self.settings.ccw_ease_in,
        ))
    }

    /// Returns the arc to ease in from the start clockwise.
    fn cw_ease_in(&self) -> Option<CircularArc> {
        Some(CircularArc::cw_from_isometry(
            self.start.isometry()?,
            self.settings.cw_ease_in,
        ))
    }

    /// Returns the arc to ease out into the goal counterclockwise.
    fn ccw_ease_out(&self) -> Option<CircularArc> {
        Some(CircularArc::ccw_from_isometry(
            self.goal.isometry()?,
            self.settings.ccw_ease_out,
        ))
    }

    /// Returns the arc to ease out into the goal clockwise.
    fn cw_ease_out(&self) -> Option<CircularArc> {
        Some(CircularArc::cw_from_isometry(
            self.goal.isometry()?,
            self.settings.cw_ease_out,
        ))
    }

    /// Checks if the `segment` collides with the [`Colliders`].
    fn collides(&self, segment: impl Into<Segment>) -> bool {
        self.colliders.intersects(segment.into())
    }
}

/// The states used by the A* algorithm.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
enum State {
    /// The start state.
    Start,
    /// The goal state.
    Goal,
    /// Easing into the path from the start counterclockwise.
    ///
    /// If start is an isometry, this state succeeds [`State::Start`].
    CcwEaseIn,
    /// Easing into the path from the start clockwise.
    ///
    /// If start is an isometry, this state succeeds [`State::Start`].
    CwEaseIn,
    /// Moving along an arc collider counterclockwise.
    ///
    /// The collider is identified by an index, and the start angle is determined.
    CcwAlongCollider(usize, OrderedFloat<f32>),
    /// Walking along an arc collider clockwise.
    ///
    /// The collider is identified by an index, and the start angle is determined.
    CwAlongCollider(usize, OrderedFloat<f32>),
    /// Easing out of the path into the goal counterclockwise.
    ///
    /// If goal is an isometry, this state precedes [`State::Goal`].
    CcwEaseOut(OrderedFloat<f32>),
    /// Easing out of the path into the goal clockwise.
    ///
    /// If goal is an isometry, this state precedes [`State::Goal`].
    CwEaseOut(OrderedFloat<f32>),
}

/// A position with an optional direction, expressed as a point or isometry.
#[derive(Copy, Clone, Debug)]
pub enum Position {
    Isometry(Isometry),
    Point(Point),
}

impl Position {
    /// Discards the direction and returns the point this position represents.
    #[must_use]
    pub fn to_point(self) -> Point {
        match self {
            Self::Isometry(isometry) => isometry.translation.vector.into(),
            Self::Point(point) => point,
        }
    }

    /// If this position is an isometry, returns it.
    #[must_use]
    pub fn isometry(self) -> Option<Isometry> {
        match self {
            Self::Isometry(isometry) => Some(isometry),
            Self::Point(_) => None,
        }
    }

    /// If this position is a point, returns it.
    #[must_use]
    pub fn point(self) -> Option<Point> {
        match self {
            Self::Isometry(_) => None,
            Self::Point(point) => Some(point),
        }
    }
}

impl From<Isometry> for Position {
    fn from(isometry: Isometry) -> Self {
        Self::Isometry(isometry)
    }
}

impl From<Point> for Position {
    fn from(point: Point) -> Self {
        Self::Point(point)
    }
}
