use bevy::prelude::*;
use nalgebra::Point2;

use crate::{
    behavior::{
        behaviors::{Observe, WalkToRL},
        engine::{CommandsBehaviorExt, RoleState, Roles, in_role},
    },
    core::config::{layout::LayoutConfig, showtime::PlayerConfig},
    motion::step_planner::{StepPlanner, Target},
};

use std::time::Duration;

/// Plugin for the Defender role
pub struct DefenderRolePlugin;

impl Plugin for DefenderRolePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, defender_role.run_if(in_role::<Defender>));
    }
}

/// The [`Defender`] role is held by any robot that does not see the ball.
/// It's job is to observe it's set position depending on player number.
#[derive(Resource)]
pub struct Defender;
impl Roles for Defender {
    const STATE: RoleState = RoleState::Defender;
}

#[derive(Resource)]
pub struct ObserveTimer {
    timer: Timer,
}

impl ObserveTimer {
    #[must_use]
    pub fn new(duration: Duration) -> Self {
        ObserveTimer {
            timer: Timer::new(duration, TimerMode::Once),
        }
    }
}

#[derive(Resource)]
pub struct StandTimer {
    timer: Timer,
}

impl StandTimer {
    #[must_use]
    pub fn new(duration: Duration) -> Self {
        StandTimer {
            timer: Timer::new(duration, TimerMode::Once),
        }
    }
}

pub fn defender_role(
    mut commands: Commands,
    player_config: Res<PlayerConfig>,
    layout_config: Res<LayoutConfig>,
    step_planner: ResMut<StepPlanner>,
    observe_timer: Option<ResMut<ObserveTimer>>,
    stand_timer: Option<ResMut<StandTimer>>,
    time: Res<Time>,
) {
    let set_robot_position = layout_config
        .set_positions
        .player(player_config.player_number);
    let set_position = set_robot_position.isometry.translation.vector;
    let set_point = Point2::new(set_position.x, set_position.y);
    let defend_target = Target {
        position: set_point,
        rotation: Some(set_robot_position.isometry.rotation),
    };

    commands.set_behavior(WalkToRL {
        target: defend_target,
    });
    return;

    // if !step_planner.has_target() {
    //     commands.set_behavior(WalkToRL {
    //         target: defend_target,
    //     });
    //     return;
    // }

    // if step_planner.reached_target() {
    //     if let Some(mut timer) = observe_timer {
    //         timer.timer.tick(time.delta());
    //         if timer.timer.finished() {
    //             commands.remove_resource::<ObserveTimer>();
    //             commands.insert_resource(StandTimer::new(Duration::from_secs(5)));
    //             commands.set_behavior(Observe { step: None });
    //         } else {
    //             commands.set_behavior(Observe::with_turning(-0.4));
    //         }
    //     } else if let Some(mut timer) = stand_timer {
    //         timer.timer.tick(time.delta());
    //         if timer.timer.finished() {
    //             commands.remove_resource::<StandTimer>();
    //             commands.insert_resource(ObserveTimer::new(Duration::from_secs(5)));
    //             commands.set_behavior(Observe::with_turning(-0.4));
    //         } else {
    //             commands.set_behavior(Observe { step: None });
    //         }
    //     } else {
    //         commands.insert_resource(ObserveTimer::new(Duration::from_secs(5)));
    //     }
    // } else {
    //     commands.set_behavior(WalkToRL {
    //         target: defend_target,
    //     });
    // }
}
