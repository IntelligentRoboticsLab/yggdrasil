use bevy::prelude::*;
use bevy::utils::Duration;
use nalgebra::Point2;

use crate::{
    behavior::{
        behaviors::{Observe, WalkTo},
        engine::{CommandsBehaviorExt, RoleState, Roles, in_role},
    },
    localization::RobotPose, // May still be useful depending on exact needs
    motion::step_planner::{StepPlanner, Target},
    vision::ball_detection::ball_tracker::BallTracker,
};

// --- Constants ---

// Define the search waypoints directly as a const array
const SEARCH_WAYPOINTS: [Point2<f32>; 5] = [
    Point2::new(0.0, 0.0),  // Center
    Point2::new(1.5, 1.0),  // Approx mid-field right
    Point2::new(0.0, 1.5),  // Approx mid-field forward-center
    Point2::new(-1.5, 1.0), // Approx mid-field left
    Point2::new(0.0, -1.5), // Approx mid-field backward-center
];

/// How long to observe (spin) at each waypoint before moving to the next (in seconds)
const OBSERVE_DURATION_SECS: u64 = 5;
/// Turning speed during observation (radians per step/update)
const OBSERVE_TURNING_SPEED: f32 = -0.6;

// --- State Resource ---

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum DefenderStatus {
    WalkingToPoint,
    ObservingAtPoint,
}

#[derive(Resource, Debug)]
struct DefenderState {
    /// Index into the `SEARCH_WAYPOINTS` array
    current_waypoint_index: usize,
    /// Current action being performed
    status: DefenderStatus,
    /// Timer to track observation duration
    observation_timer: Timer,
}

// --- Plugin Definition ---
/// Plugin for the Defender role (with multi-point search behavior)
pub struct DefenderRolePlugin;

impl Plugin for DefenderRolePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, defender_role.run_if(in_role::<Defender>))
            // Add systems to manage DefenderState on role entry/exit
            .add_systems(OnEnter(RoleState::Defender), add_defender_state)
            .add_systems(OnExit(RoleState::Defender), remove_defender_state);
    }
}

/// Add the state resource when entering the Defender role
fn add_defender_state(mut commands: Commands) {
    commands.insert_resource(DefenderState {
        current_waypoint_index: 0,
        status: DefenderStatus::WalkingToPoint, // Start by walking
        observation_timer: Timer::new(Duration::from_secs(OBSERVE_DURATION_SECS), TimerMode::Once),
    });
}

/// Remove the state resource when exiting the Defender role
fn remove_defender_state(mut commands: Commands) {
    commands.remove_resource::<DefenderState>();
}

// --- Role Definition ---
/// The [`Defender`] role patrols between waypoints, observing at each, to find the ball.
#[derive(Resource)]
pub struct Defender;

impl Roles for Defender {
    const STATE: RoleState = RoleState::Defender;
}

// --- System Implementation ---
#[allow(clippy::too_many_arguments)]
pub fn defender_role(
    mut commands: Commands,
    mut defender_state_opt: Option<ResMut<DefenderState>>,
    mut step_planner: ResMut<StepPlanner>,
    ball_tracker: Res<BallTracker>,
    time: Res<Time>,      // Or Res<Time>
    pose: Res<RobotPose>, // Keep if needed for other logic or checks
) {
    // 1. Check if ball is visible. If so, role should change. Do nothing.
    if ball_tracker.stationary_ball().is_some() {
        // If ball found, potentially stop motion immediately
        // commands.set_behavior(Idle); // Or similar stop behavior
        return;
    }

    // 2. Get the state resource. If not present, do nothing (wait for OnEnter).
    let Some(mut state) = defender_state_opt else {
        return;
    };

    // Check if waypoints list is empty (shouldn't be with const array)
    if SEARCH_WAYPOINTS.is_empty() {
        commands.set_behavior(Observe::with_turning(OBSERVE_TURNING_SPEED));
        return;
    }

    // 3. Get the current target waypoint
    let current_index = state.current_waypoint_index % SEARCH_WAYPOINTS.len();
    let current_waypoint = SEARCH_WAYPOINTS[current_index];

    let target = Target {
        position: current_waypoint,
        rotation: None,
    };

    // 4. Execute logic based on current status
    match state.status {
        DefenderStatus::WalkingToPoint => {
            commands.set_behavior(WalkTo { target });

            // Check if we've arrived at the *correct* target
            if step_planner.has_target()
                && step_planner.target.unwrap().position == current_waypoint
                && step_planner.reached_target()
            {
                // Arrived: Switch to Observing
                state.status = DefenderStatus::ObservingAtPoint;
                state
                    .observation_timer
                    .set_duration(Duration::from_secs(OBSERVE_DURATION_SECS));
                state.observation_timer.reset();
                commands.set_behavior(Observe::with_turning(OBSERVE_TURNING_SPEED));
            }
        }
        DefenderStatus::ObservingAtPoint => {
            commands.set_behavior(Observe::with_turning(OBSERVE_TURNING_SPEED));

            // Tick the timer
            state.observation_timer.tick(time.delta());

            // Check if observation time is up
            if state.observation_timer.finished() {
                // Time's up: Switch back to Walking
                state.status = DefenderStatus::WalkingToPoint;
                // Increment waypoint index, wrapping around
                state.current_waypoint_index = (current_index + 1) % SEARCH_WAYPOINTS.len();
                let next_waypoint = SEARCH_WAYPOINTS[state.current_waypoint_index];
                let next_target = Target {
                    position: next_waypoint,
                    rotation: None,
                };
                // Set behavior to walk to the *next* waypoint
                commands.set_behavior(WalkTo {
                    target: next_target,
                });
            }
        }
    }
}
