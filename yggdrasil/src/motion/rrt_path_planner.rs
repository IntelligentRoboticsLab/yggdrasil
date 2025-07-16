use core::f32::consts::PI;
use nalgebra::{Point2, Vector2, Vector3};
use rand::rngs::OsRng;
use rand::{Rng, SeedableRng, rngs::StdRng};

//
// ------------------------------------------------------------
// Shared angle & SE(2) utilities
// ------------------------------------------------------------
//

/// Wrap an angle to [-π, π].
#[inline(always)]
pub fn wrap_to_pi(angle: f32) -> f32 {
    let two_pi = 2.0 * PI;
    let mut a = (angle + PI).rem_euclid(two_pi) - PI;
    if a <= -PI { a + two_pi } else { a }
}

/// SE(2) distance with unit weights on x,y,θ.
/// (Adjust weighting if you want heading tighter/looser.)
#[inline(always)]
pub fn se2_distance(a: &Vector3<f32>, b: &Vector3<f32>) -> f32 {
    let dx = a[0] - b[0];
    let dy = a[1] - b[1];
    let dtheta = wrap_to_pi(a[2] - b[2]);
    (dx * dx + dy * dy + dtheta * dtheta).sqrt()
}

/// Steer toward `to` from `from`, capped to `step_size`.
#[inline(always)]
pub fn se2_steer(from: &Vector3<f32>, to: &Vector3<f32>, step_size: f32) -> Vector3<f32> {
    let dx = to[0] - from[0];
    let dy = to[1] - from[1];
    let dtheta = wrap_to_pi(to[2] - from[2]);

    let dist = (dx * dx + dy * dy + dtheta * dtheta).sqrt();
    if dist <= step_size {
        Vector3::new(to[0], to[1], wrap_to_pi(to[2]))
    } else {
        let f = step_size / dist;
        Vector3::new(
            from[0] + dx * f,
            from[1] + dy * f,
            wrap_to_pi(from[2] + dtheta * f),
        )
    }
}

//
// ------------------------------------------------------------
// Obstacles (reuse your definition)
// ------------------------------------------------------------
//

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub struct Obstacle {
    pub x: ordered_float::OrderedFloat<f32>,
    pub y: ordered_float::OrderedFloat<f32>,
    pub radius: ordered_float::OrderedFloat<f32>,
}

impl Obstacle {
    #[must_use]
    pub fn new(x: f32, y: f32, radius: f32) -> Self {
        Self {
            x: ordered_float::OrderedFloat(x),
            y: ordered_float::OrderedFloat(y),
            radius: ordered_float::OrderedFloat(radius),
        }
    }

    #[must_use]
    pub fn distance(&self, other: &Self) -> f32 {
        let x = self.x.0 - other.x.0;
        let y = self.y.0 - other.y.0;
        (x * x + y * y).sqrt()
    }
}

/// Fast circle test (matches `obstructs()` logic you're using in A*).
#[inline(always)]
fn circle_line_intersects(ox: f32, oy: f32, r: f32, ax: f32, ay: f32, bx: f32, by: f32) -> bool {
    // Distance from circle center (ox,oy) to segment AB.
    // Standard projection clamp.
    let abx = bx - ax;
    let aby = by - ay;
    let apx = ox - ax;
    let apy = oy - ay;

    let ab_len2 = abx * abx + aby * aby;
    let t = if ab_len2 > 0.0 {
        (apx * abx + apy * aby) / ab_len2
    } else {
        0.0
    };
    let t = t.clamp(0.0, 1.0);

    let cx = ax + t * abx;
    let cy = ay + t * aby;

    let dx = ox - cx;
    let dy = oy - cy;
    let dist2 = dx * dx + dy * dy;

    dist2 < (1.01 * r) * (1.01 * r) // 1.01 factor to mirror your `obstructs()`
}

/// Point‑in‑free‑space test: outside every inflated obstacle.
#[inline(always)]
fn point_clear(x: f32, y: f32, obstacles: &[Obstacle]) -> bool {
    for o in obstacles {
        let dx = x - o.x.0;
        let dy = y - o.y.0;
        if dx * dx + dy * dy <= (1.01 * o.radius.0).powi(2) {
            return false;
        }
    }
    true
}

/// Segment free?: no obstacle intersects segment AB.
#[inline(always)]
fn segment_clear(ax: f32, ay: f32, bx: f32, by: f32, obstacles: &[Obstacle]) -> bool {
    for o in obstacles {
        if circle_line_intersects(o.x.0, o.y.0, o.radius.0, ax, ay, bx, by) {
            return false;
        }
    }
    true
}

//
// ------------------------------------------------------------
// RRT* node
// ------------------------------------------------------------
//

#[derive(Clone)]
struct Node {
    state: Vector3<f32>,
    parent: Option<usize>,
    cost: f32,
}

impl Node {
    #[inline(always)]
    fn new(state: Vector3<f32>, parent: Option<usize>, cost: f32) -> Self {
        Self {
            state,
            parent,
            cost,
        }
    }
}

//
// ------------------------------------------------------------
// Obstacle‑aware SE(2) RRT*
// ------------------------------------------------------------
//

pub struct RRTStarSE2 {
    start: Vector3<f32>,
    goal: Vector3<f32>,
    x_min: Vector3<f32>,
    x_max: Vector3<f32>,
    step_size: f32,
    search_radius: f32,
    max_iters: usize,
    goal_bias: f32,
    nodes: Vec<Node>,
    node_free: Box<dyn Fn(&Vector3<f32>) -> bool + Send + Sync>,
    edge_free: Box<dyn Fn(&Vector3<f32>, &Vector3<f32>) -> bool + Send + Sync>,
    rng: StdRng,
}

impl RRTStarSE2 {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        start: Vector3<f32>,
        goal: Vector3<f32>,
        x_min: Vector3<f32>,
        x_max: Vector3<f32>,
        step_size: f32,
        search_radius: f32,
        max_iters: usize,
        goal_bias: f32,
        node_free: impl Fn(&Vector3<f32>) -> bool + Send + Sync + 'static,
        edge_free: impl Fn(&Vector3<f32>, &Vector3<f32>) -> bool + Send + Sync + 'static,
    ) -> Self {
        let rng = StdRng::from_os_rng();
        let start = Vector3::new(start[0], start[1], wrap_to_pi(start[2]));
        let goal = Vector3::new(goal[0], goal[1], wrap_to_pi(goal[2]));
        let mut nodes = Vec::with_capacity(max_iters + 2);
        nodes.push(Node::new(start.clone(), None, 0.0));
        Self {
            start,
            goal,
            x_min,
            x_max,
            step_size,
            search_radius,
            max_iters,
            goal_bias,
            nodes,
            node_free: Box::new(node_free),
            edge_free: Box::new(edge_free),
            rng,
        }
    }

    /// Run planning. Returns path states start→goal, if found.
    pub fn plan(&mut self) -> Option<Vec<Vector3<f32>>> {
        for _ in 0..self.max_iters {
            // 1. Sample
            let x_rand = self.sample_state();

            // 2. Nearest
            let nearest_idx = self.nearest_neighbor(&x_rand);
            let x_nearest = &self.nodes[nearest_idx].state;

            // 3. Steer
            let x_new = se2_steer(x_nearest, &x_rand, self.step_size);

            // 4. Collision
            if !(self.node_free)(&x_new) {
                continue;
            }
            if !(self.edge_free)(x_nearest, &x_new) {
                continue;
            }

            // 5. Neighbors
            let neighbor_indices = self.find_neighbors(&x_new);

            // 6. Choose parent
            let mut c_min = f32::INFINITY;
            let mut parent_idx = None;
            for &idx in &neighbor_indices {
                let node = &self.nodes[idx];
                if !(self.edge_free)(&node.state, &x_new) {
                    continue;
                }
                let c_through = node.cost + se2_distance(&node.state, &x_new);
                if c_through < c_min {
                    c_min = c_through;
                    parent_idx = Some(idx);
                }
            }
            let (cost_new, parent_idx) = if let Some(pi) = parent_idx {
                (c_min, pi)
            } else {
                let node = &self.nodes[nearest_idx];
                (node.cost + se2_distance(&node.state, &x_new), nearest_idx)
            };

            // 7. Insert
            let new_idx = self.nodes.len();
            self.nodes
                .push(Node::new(x_new.clone(), Some(parent_idx), cost_new));

            // 8. Rewire
            for &idx in &neighbor_indices {
                if idx == parent_idx {
                    continue;
                }
                // Only rewire if edge is free
                if !(self.edge_free)(&x_new, &self.nodes[idx].state) {
                    continue;
                }
                let cost_through_new = cost_new + se2_distance(&self.nodes[idx].state, &x_new);
                if cost_through_new < self.nodes[idx].cost {
                    self.nodes[idx].parent = Some(new_idx);
                    self.nodes[idx].cost = cost_through_new;
                }
            }

            // 9. Try goal
            if se2_distance(&x_new, &self.goal) <= self.step_size
                && (self.node_free)(&self.goal)
                && (self.edge_free)(&x_new, &self.goal)
            {
                let goal_cost = cost_new + se2_distance(&x_new, &self.goal);
                let goal_idx = self.nodes.len();
                self.nodes
                    .push(Node::new(self.goal.clone(), Some(new_idx), goal_cost));
                return Some(self.extract_path(goal_idx));
            }
        }
        None
    }

    #[inline]
    fn sample_state(&mut self) -> Vector3<f32> {
        if self.rng.gen_bool(self.goal_bias as f64) {
            return self.goal.clone();
        }
        let mut s = Vector3::zeros();
        for i in 0..3 {
            s[i] = self.rng.gen_range(self.x_min[i]..self.x_max[i]);
        }
        s[2] = wrap_to_pi(s[2]);
        s
    }

    #[inline]
    fn nearest_neighbor(&self, x: &Vector3<f32>) -> usize {
        let mut best = 0;
        let mut best_d = f32::INFINITY;
        for (i, n) in self.nodes.iter().enumerate() {
            let d = se2_distance(&n.state, x);
            if d < best_d {
                best_d = d;
                best = i;
            }
        }
        best
    }

    #[inline]
    fn find_neighbors(&self, x: &Vector3<f32>) -> Vec<usize> {
        let mut out = Vec::new();
        for (i, n) in self.nodes.iter().enumerate() {
            if se2_distance(&n.state, x) <= self.search_radius {
                out.push(i);
            }
        }
        out
    }

    fn extract_path(&self, mut idx: usize) -> Vec<Vector3<f32>> {
        let mut path = Vec::new();
        loop {
            path.push(self.nodes[idx].state.clone());
            if let Some(p) = self.nodes[idx].parent {
                idx = p;
            } else {
                break;
            }
        }
        path.reverse();
        path
    }
}

//
// ------------------------------------------------------------
// Helpers: build closures from Obstacle slice
// ------------------------------------------------------------
//

/// Build collision functors from a slice of circular obstacles.
/// θ is ignored (XY only).
pub fn make_obstacle_checkers(
    obstacles: Vec<Obstacle>,
) -> (
    impl Fn(&Vector3<f32>) -> bool + Send + Sync,
    impl Fn(&Vector3<f32>, &Vector3<f32>) -> bool + Send + Sync,
) {
    let obstacles = obstacles.into_boxed_slice();
    // Node free: point is clear of all obstacles.
    let obstacles_clone = obstacles.clone();
    let node_free = move |s: &Vector3<f32>| point_clear(s[0], s[1], &obstacles_clone);
    let edge_free =
        move |a: &Vector3<f32>, b: &Vector3<f32>| segment_clear(a[0], a[1], b[0], b[1], &obstacles);
    (node_free, edge_free)
}

//
// ------------------------------------------------------------
// Convenience: run planner directly from 2D inputs
// ------------------------------------------------------------
//

/// Quick wrapper to run SE(2) RRT* given 2D start/goal + yaw seeds.
/// Bounding box auto‑computed from obstacles w/ margin (or override).
pub fn plan_se2_with_obstacles(
    start_xy: Point2<f32>,
    start_yaw: f32,
    goal_xy: Point2<f32>,
    goal_yaw: f32,
    obstacles: Vec<Obstacle>,
    step_size: f32,
    search_radius: f32,
    max_iters: usize,
    goal_bias: f32,
    // Optional bounds override: if None, derive from obstacles + start/goal + margin
    bounds: Option<(Vector3<f32>, Vector3<f32>)>,
) -> Option<Vec<Vector3<f32>>> {
    let (x_min, x_max) = if let Some(b) = bounds {
        b
    } else {
        derive_bounds(start_xy, goal_xy, &obstacles, 0.5) // 0.5m margin; tweak
    };

    let start = Vector3::new(start_xy.x, start_xy.y, wrap_to_pi(start_yaw));
    let goal = Vector3::new(goal_xy.x, goal_xy.y, wrap_to_pi(goal_yaw));

    let (node_free, edge_free) = make_obstacle_checkers(obstacles.clone());

    let mut planner = RRTStarSE2::new(
        start,
        goal,
        x_min,
        x_max,
        step_size,
        search_radius,
        max_iters,
        goal_bias,
        node_free,
        edge_free,
    );

    planner.plan()
}

/// Derive bounding box from start, goal, and all obstacles + margin.
/// θ bounds always [-π, π].
fn derive_bounds(
    start: Point2<f32>,
    goal: Point2<f32>,
    obstacles: &[Obstacle],
    margin: f32,
) -> (Vector3<f32>, Vector3<f32>) {
    let mut xmin = start.x.min(goal.x);
    let mut xmax = start.x.max(goal.x);
    let mut ymin = start.y.min(goal.y);
    let mut ymax = start.y.max(goal.y);

    for o in obstacles {
        let r = o.radius.0;
        xmin = xmin.min(o.x.0 - r);
        xmax = xmax.max(o.x.0 + r);
        ymin = ymin.min(o.y.0 - r);
        ymax = ymax.max(o.y.0 + r);
    }

    xmin -= margin;
    xmax += margin;
    ymin -= margin;
    ymax += margin;

    (Vector3::new(xmin, ymin, -PI), Vector3::new(xmax, ymax, PI))
}

//
// ------------------------------------------------------------
// Optional: rebuild headings from 2D path (if you don't trust sampled θ)
// ------------------------------------------------------------
//

/// Given an XY path, assign heading along segment tangent.
/// If `preserve_endpoints` true, keep original start/goal yaw values and only fill interior.
/// Otherwise compute for all (start uses next segment, goal uses prev segment).
pub fn assign_headings_from_xy(
    path_xy: &[Point2<f32>],
    start_yaw: f32,
    goal_yaw: f32,
    preserve_endpoints: bool,
) -> Vec<Vector3<f32>> {
    assert!(path_xy.len() >= 2);
    let mut out = Vec::with_capacity(path_xy.len());

    for i in 0..path_xy.len() {
        let yaw = if preserve_endpoints {
            if i == 0 {
                wrap_to_pi(start_yaw)
            } else if i + 1 == path_xy.len() {
                wrap_to_pi(goal_yaw)
            } else {
                heading_between(&path_xy[i], &path_xy[i + 1])
            }
        } else {
            if i + 1 < path_xy.len() {
                heading_between(&path_xy[i], &path_xy[i + 1])
            } else {
                heading_between(&path_xy[i - 1], &path_xy[i])
            }
        };
        out.push(Vector3::new(path_xy[i].x, path_xy[i].y, yaw));
    }
    out
}

#[inline(always)]
fn heading_between(a: &Point2<f32>, b: &Point2<f32>) -> f32 {
    wrap_to_pi((b.y - a.y).atan2(b.x - a.x))
}
