use geo::{ClosestPoint, Coord, Line};
use pathfinding::directed::astar;

use nalgebra::{base::Vector2, Point2};

use std::hash::Hash;

use ordered_float::OrderedFloat;

#[derive(Copy, Clone, Eq, Debug, PartialEq)]
struct Point {
    x: OrderedFloat<f32>,
    y: OrderedFloat<f32>,

    parent_obstacle: Option<Obstacle>,
}

impl Hash for Point {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.x.hash(state);
        self.y.hash(state);
    }
}

impl PartialOrd for Point {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Point {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self.x.cmp(&other.x) {
            core::cmp::Ordering::Equal => {}
            ord => return ord,
        }
        self.y.cmp(&other.y)
    }
}

impl From<Point> for Coord {
    fn from(value: Point) -> Self {
        Coord {
            x: value.x.0 as f64,
            y: value.y.0 as f64,
        }
    }
}

impl From<&Point> for Coord {
    fn from(value: &Point) -> Self {
        Coord {
            x: value.x.0 as f64,
            y: value.y.0 as f64,
        }
    }
}

fn side_points(point: &Point, obstacle: &Obstacle) -> [Point; 2] {
    let to_point = Vector2::new(point.x.0, point.y.0);
    let point_to_obstacle = Vector2::new(obstacle.x.0 - point.x.0, obstacle.y.0 - point.y.0);
    let mut obstacle_to_side = Vector2::new(-(obstacle.y.0 - point.y.0), obstacle.x.0 - point.x.0);
    obstacle_to_side = obstacle_to_side.normalize();
    obstacle_to_side *= obstacle.radius.0;

    let point1 = to_point + point_to_obstacle + obstacle_to_side;
    let point2 = to_point + point_to_obstacle - obstacle_to_side;

    [
        Point::from_obstacle(point1.x, point1.y, *obstacle),
        Point::from_obstacle(point2.x, point2.y, *obstacle),
    ]
}

fn obstructs(obstacle: &Obstacle, line: geo::Line) -> bool {
    let point = geo::geometry::Point::new(obstacle.x.0 as f64, obstacle.y.0 as f64);
    match line.closest_point(&point) {
        geo::Closest::Intersection(_) => true,
        geo::Closest::SinglePoint(point2) => {
            obstacle.radius.0 as f64
                > 1.01
                    * nalgebra::distance(
                        &nalgebra::Point2::new(point.x(), point.y()),
                        &nalgebra::Point2::new(point2.x(), point2.y()),
                    )
        }
        geo::Closest::Indeterminate => unreachable!(),
    }
}

impl Point {
    pub fn new(x: f32, y: f32) -> Self {
        Self {
            x: OrderedFloat(x),
            y: OrderedFloat(y),
            parent_obstacle: None,
        }
    }

    pub fn from_obstacle(x: f32, y: f32, obstacle: Obstacle) -> Self {
        Self {
            x: OrderedFloat(x),
            y: OrderedFloat(y),
            parent_obstacle: Some(obstacle),
        }
    }

    pub fn distance(&self, other: &Self) -> OrderedFloat<f32> {
        OrderedFloat(((self.x - other.x).powi(2) + (self.y - other.y).powi(2)).sqrt())
    }

    pub fn successors(
        &self,
        all_obstacles: &[Obstacle],
        goal: Point,
    ) -> Vec<(Self, OrderedFloat<f32>)> {
        let mut successors = Vec::with_capacity(all_obstacles.len() * 2);

        if !self.gets_obstructed(&goal, all_obstacles) {
            successors.push((goal, self.distance(&goal)));
        }

        for obstacle in all_obstacles {
            for side_point in &side_points(self, obstacle) {
                if !self.gets_obstructed(side_point, all_obstacles) {
                    successors.push((*side_point, self.distance(side_point)));
                }
            }
        }

        successors
    }

    fn gets_obstructed(&self, destination: &Self, all_obstacles: &[Obstacle]) -> bool {
        for obstacle in all_obstacles {
            // if !self
            //     .parent_obstacle
            //     .is_some_and(|parent_obstacle| parent_obstacle == *obstacle)
            //     && !destination
            //         .parent_obstacle
            //         .is_some_and(|parent_obstacle| parent_obstacle == *obstacle)
            //     && obstructs(obstacle, Line::new(self, destination))
            // {
            //     return true;
            // }
            if !destination
                .parent_obstacle
                .is_some_and(|parent_obstacle| parent_obstacle == *obstacle)
                && obstructs(obstacle, Line::new(self, destination))
            {
                return true;
            }
        }

        false
    }
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub struct Obstacle {
    x: OrderedFloat<f32>,
    y: OrderedFloat<f32>,
    radius: OrderedFloat<f32>,
}

impl Obstacle {
    pub fn new(x: f32, y: f32, radius: f32) -> Self {
        Self {
            x: OrderedFloat(x),
            y: OrderedFloat(y),
            radius: OrderedFloat(radius),
        }
    }

    pub fn distance(&self, other: &Self) -> f32 {
        let x = self.x.0 - other.x.0;
        let y = self.y.0 - other.y.0;
        (x * x + y * y).sqrt()
    }
}

pub fn find_path(
    start: Point2<f32>,
    goal: Point2<f32>,
    obstacles: &[Obstacle],
) -> Option<(Vec<Point2<f32>>, f32)> {
    let start = Point::new(start.x, start.y);
    let goal = Point::new(goal.x, goal.y);

    for obstacle in obstacles {
        let center = Point::new(obstacle.x.0, obstacle.y.0);
        if goal.distance(&center) <= obstacle.radius {
            return None;
        }
    }

    let result = astar::astar(
        &start,
        |point| point.successors(obstacles, goal),
        |point| point.distance(&goal),
        |obstacle| obstacle.eq(&goal),
    );

    result.map(|(path, cost)| {
        (
            path.iter()
                .map(|point| Point2::new(point.x.0, point.y.0))
                .collect(),
            cost.0,
        )
    })
}
