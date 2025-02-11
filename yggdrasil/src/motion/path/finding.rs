//! Lower-level pathfinding capabilities.

use nalgebra as na;
use ordered_float::OrderedFloat;

use super::geometry::{Connection, Winding};
use super::{
    geometry::{
        Ccw, CircularArc, Cw, Intersects, Isometry, Length, LineSegment, Node, Point, Segment,
    },
    PathConfig,
};

type Float = OrderedFloat<f32>;

/// Struct containing all the data necessary for pathfinding.
#[derive(Copy, Clone)]
pub struct Pathfinding<'a> {
    /// The start position of the path.
    pub start: Target,
    /// The target position of the path.
    pub target: Target,
    /// The colliders to navigate around.
    pub colliders: &'a Colliders,
    /// The settings for pathfinding.
    pub config: &'a PathConfig,
}

impl Pathfinding<'_> {
    /// Calculates the shortest path from `start` to `target` and returns the segments that make up
    /// the path as well as the total length (if such a path exists).
    #[must_use]
    pub fn path(&self) -> Option<(Vec<Segment>, f32)> {
        let (states, cost) = self.astar()?;
        let mut segments = Vec::new();

        for window in states.windows(2) {
            let (prev, next) = match *window {
                [State::Start, State::EaseIn(_)] => continue,
                [prev @ State::EaseOut(_, _), _] => {
                    segments.push(self.arc(prev).unwrap().into());
                    break;
                }
                [prev, next] => (prev, next),
                _ => unreachable!(),
            };

            Connection::new(self.node(prev), self.node(next))
                .unwrap()
                .for_each_determined(|s| segments.push(s));
        }

        Some((segments, cost.into()))
    }

    /// Finds the shortest path using the A* algorithm.
    fn astar(&self) -> Option<(Vec<State>, Float)> {
        pathfinding::directed::astar::astar(
            &State::Start,
            |state| self.successors(*state),
            |state| self.heuristic(*state),
            |state| matches!(state, State::Goal),
        )
    }

    /// Returns the heuristic for A* based on the euclidean distance to the target.
    fn heuristic(&self, state: State) -> Float {
        na::distance(&self.point(state), &self.target.to_point()).into()
    }

    /// Returns the successors of a given state.
    fn successors(&self, state: State) -> Vec<(State, Float)> {
        match state {
            State::Goal => unreachable!(),
            State::EaseOut(_, _) => {
                let arc = self.arc(state).unwrap();

                (!self.colliders.intersects(Segment::from(arc)))
                    .then_some(State::Goal.with_cost(arc))
                    .into_iter()
                    .collect()
            }
            State::Start => self.start_successors(),
            state => self.node_successors(self.node(state)),
        }
    }

    /// Returns the successors from [`State::Start`].
    fn start_successors(&self) -> Vec<(State, Float)> {
        match self.start {
            Target::Isometry(_) => vec![
                State::EaseIn(Ccw).without_cost(),
                State::EaseIn(Cw).without_cost(),
            ],
            Target::Point(start) => self.node_successors(start.into()),
        }
    }

    /// Returns the successors from a node.
    fn node_successors(&self, prev: Node) -> Vec<(State, Float)> {
        let along_collider = |connection: Connection, index| {
            let next = connection.next.unwrap();
            State::AlongCollider(index, next.direction(), next.start.into())
        };

        // Potential successors that directly lead to the target.
        let direct: [(Option<Node>, fn(Connection) -> State); 3] = [
            (self.target.point().map(Node::from), |_| State::Goal),
            (self.ease_out(Ccw).map(Node::from), |connection| {
                State::EaseOut(Ccw, connection.next.unwrap().start.into())
            }),
            (self.ease_out(Winding::Cw).map(Node::from), |connection| {
                State::EaseOut(Cw, connection.next.unwrap().start.into())
            }),
        ];

        let direct = direct
            .into_iter()
            .filter_map(|(next, to_state)| Some((next?, to_state)))
            .filter_map(|(next, to_state)| {
                Connection::new(prev, next)
                    .filter(|connection| !self.colliders.intersects(*connection))
                    .map(|connection| to_state(connection).with_cost(connection))
            });

        // Potential successors that don't directly lead to the target.
        let indirect = self
            .colliders
            .arcs
            .iter()
            .enumerate()
            .flat_map(|(index, next)| [(index, *next), (index, next.flip())])
            .filter_map(|(index, next)| {
                Connection::new(prev, next)
                    .filter(|connection| !self.colliders.intersects(*connection))
                    .map(|connection| along_collider(connection, index).with_cost(connection))
            });

        direct.chain(indirect).collect()
    }

    /// Get the [`Node`] this [`State`] represents.
    fn node(&self, state: State) -> Node {
        match self.arc(state) {
            Some(arc) => arc.into(),
            None => self.point(state).into(),
        }
    }

    /// Get the `Point` this [`State`] represents.
    fn point(&self, state: State) -> Point {
        match state {
            State::Start => self.start.to_point(),
            State::Goal => self.target.to_point(),
            state => self.arc(state).unwrap().point_at_start(),
        }
    }

    /// Get the [`CircularArc`] this [`State`] represents.
    fn arc(&self, state: State) -> Option<CircularArc> {
        match state {
            State::Start => None,
            State::Goal => None,
            State::EaseIn(direction) => self.ease_in(direction),
            State::AlongCollider(index, direction, start) => self.colliders.arcs[index]
                .with_direction(direction)
                .enter(start.into()),
            State::EaseOut(direction, start) => {
                self.ease_out(direction)?.enter_non_circular(start.into())
            }
        }
    }

    /// Returns the arc to ease in from the start.
    fn ease_in(&self, direction: Winding) -> Option<CircularArc> {
        Some(CircularArc::from_isometry(
            self.start.isometry()?,
            direction,
            self.config.ease_in,
        ))
    }

    /// Returns the arc to ease in from the start.
    fn ease_out(&self, direction: Winding) -> Option<CircularArc> {
        Some(CircularArc::from_isometry(
            self.target.isometry()?,
            direction,
            self.config.ease_out,
        ))
    }
}

/// The states used by the A* algorithm.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
enum State {
    /// The start state.
    Start,
    /// The target state.
    Goal,
    /// Easing into the path from the start.
    ///
    /// If start is an isometry, this state succeeds [`State::Start`].
    EaseIn(Winding),
    /// Moving along an arc collider.
    ///
    /// The collider is identified by an index, and the start angle is determined.
    AlongCollider(usize, Winding, OrderedFloat<f32>),
    /// Easing out of the path into the target.
    ///
    /// If target is an isometry, this state precedes [`State::Goal`].
    EaseOut(Winding, OrderedFloat<f32>),
}

impl State {
    fn with_cost(self, cost: impl Length) -> (Self, Float) {
        (self, cost.length().into())
    }

    fn without_cost(self) -> (Self, Float) {
        (self, 0.0.into())
    }
}

/// A target with an optional direction, expressed as a point or an isometry.
#[derive(Copy, Clone, Debug)]
pub enum Target {
    Isometry(Isometry),
    Point(Point),
}

impl Target {
    /// Returns the distance between two [`Target`]s.
    #[must_use]
    pub fn distance(self, other: Target) -> f32 {
        na::distance(&self.to_point(), &other.to_point())
    }

    /// Returns the angular distance between two [`Target`]s.
    #[must_use]
    pub fn angular_distance(self, other: Target) -> Option<f32> {
        Some(other.angle()? - self.angle()?)
    }

    /// Returns the angle of this [`Target`].
    #[must_use]
    pub fn angle(self) -> Option<f32> {
        Some(self.isometry()?.rotation.angle())
    }

    /// Discards the direction and returns the point this position represents.
    #[must_use]
    pub fn to_point(self) -> Point {
        match self {
            Self::Isometry(isometry) => isometry.translation.vector.into(),
            Self::Point(point) => point,
        }
    }

    /// Converts a [`Target::Isometry`] to a [`Target::Point`].
    #[must_use]
    pub fn isometry_to_point(self) -> Option<Self> {
        Some(Self::Point(self.isometry()?.translation.vector.into()))
    }

    /// If this target is an isometry, returns it.
    #[must_use]
    pub fn isometry(self) -> Option<Isometry> {
        match self {
            Self::Isometry(isometry) => Some(isometry),
            Self::Point(_) => None,
        }
    }

    /// If this target is a point, returns it.
    #[must_use]
    pub fn point(self) -> Option<Point> {
        match self {
            Self::Isometry(_) => None,
            Self::Point(point) => Some(point),
        }
    }
}

impl From<Isometry> for Target {
    fn from(isometry: Isometry) -> Self {
        Self::Isometry(isometry)
    }
}

impl From<Point> for Target {
    fn from(point: Point) -> Self {
        Self::Point(point)
    }
}

/// The colliders that the pathfinding navigates around, consists of circular arcs and line
/// segments.
#[derive(Clone, Default, PartialEq)]
pub struct Colliders {
    pub arcs: Vec<CircularArc>,
    pub lines: Vec<LineSegment>,
}

impl Colliders {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            arcs: Vec::new(),
            lines: Vec::new(),
        }
    }

    /// Iterates over the segments that this struct contains.
    pub fn segments(&self) -> impl Iterator<Item = Segment> + '_ {
        let arcs = self.arcs.iter().copied().map(Into::into);
        let lines = self.lines.iter().copied().map(Into::into);

        arcs.chain(lines)
    }
}

impl Intersects<Segment> for &Colliders {
    type Intersection = bool;

    fn intersects(self, segment: Segment) -> bool {
        for other in self.segments() {
            if segment.intersects(other) {
                return true;
            }
        }

        false
    }
}
