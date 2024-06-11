//! Functions for smoothing out the various trajectories in the walking engine.

use std::f32::consts::{FRAC_PI_2, PI};

/// Returns a value between 0 and 1, where 0 is the start of the path and 1 is the end.
///
/// The path is a parabola, where the start and end are at the same height.
/// Visualised in Desmos: [link](https://www.desmos.com/calculator/kw2ywp6qvh)
///
/// # Examples
/// ```no_run
/// use yggdrasil::motion::walk::smoothing::parabolic_return;
///
/// assert_eq!(parabolic_return(0.0), 0.0);
/// assert_eq!(parabolic_return(0.5), 1.0);
/// assert_eq!(parabolic_return(1.0), 0.0);
/// ```
pub fn parabolic_return(t: f32) -> f32 {
    0.5 * (2.0 * PI * t - FRAC_PI_2).sin() + 0.5
}

/// Returns a value between 0 and 1, where 0 is the start of the path and 1 is the end.
///
/// This path is a parabolic scale that starts at 0 and ends at 1, and at 0.5 at the halfway point.
/// Visualised in Desmos: [link](https://www.desmos.com/calculator/fwyo4ggnyy)
///
/// # Examples
/// ```no_run
/// use yggdrasil::motion::walk::smoothing::parabolic_step;
///
/// assert_eq!(parabolic_step(0.0), 0.0);
/// assert_eq!(parabolic_step(0.5), 0.5);
/// assert_eq!(parabolic_step(1.0), 1.0);
/// ```
pub fn parabolic_step(t: f32) -> f32 {
    if t < 0.5 {
        2.0 * t.powi(2)
    } else {
        4.0 * t - 2.0 * t.powi(2) - 1.0
    }
}
