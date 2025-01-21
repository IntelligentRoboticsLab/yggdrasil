use bevy::prelude::*;
use bifrost::communication::{GameControllerMessage, GamePhase};
use heimdall::{Bottom, Top};
use nalgebra::Point2;

use crate::{
    behavior::{
        behaviors::{Sitting, Stand, StandLookAt, Standup, WalkToSet},
        engine::{in_role, CommandsBehaviorExt},
        primary_state::PrimaryState,
        roles::Striker,
    },
    motion::walk::engine::WalkingEngine,
    sensor::{button::HeadButtons, falling::FallState},
    vision::ball_detection::classifier::Balls,
};

use crate::behavior::engine::{BehaviorState, Role, Roles};

/// Plugin for the Instinct role
pub struct InstinctRolePlugin;

impl Plugin for InstinctRolePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, formation_role.run_if(in_role::<Instinct>));
    }
}

/// The [`Instinct`] role is held by a single robot at a time, usually player number 1.
/// It's job is to prevent the ball from entering the goal, which it does by staying in the goal area.
#[derive(Resource)]
pub struct Instinct;
impl Roles for Instinct {
    const STATE: Role = Role::Instinct;
}

#[allow(clippy::too_many_arguments)]
pub fn formation_role(
    mut commands: Commands,
    state: Res<State<BehaviorState>>,
    walking_engine: Res<WalkingEngine>,
    head_buttons: Res<HeadButtons>,
    primary_state: Res<PrimaryState>,
    game_controller_message: Option<Res<GameControllerMessage>>,
    fall_state: Res<FallState>,
    top_balls: Res<Balls<Top>>,
    bottom_balls: Res<Balls<Bottom>>,
) {
    let behavior = state.get();

    if state.get() == &BehaviorState::StartUp {
        if walking_engine.is_sitting() || head_buttons.all_pressed() {
            commands.set_behavior(Sitting);
        }
        if *primary_state == PrimaryState::Initial {
            commands.set_behavior(Stand);
        }
        return;
    }

    // unstiff has the number 1 precedence
    if *primary_state == PrimaryState::Sitting {
        commands.set_behavior(Sitting);
        return;
    }

    // if BehaviorState::Standup(standup) == self.behavior {
    //     if standup.completed() {
    //         self.behavior = self.prev_behavior_for_standup.take().unwrap();
    //     }
    //     return;
    // }

    // next up, damage prevention and standup motion take precedence
    match fall_state.as_ref() {
        FallState::Lying(_) => {
            commands.set_behavior(Standup::default());

            return;
        }
        FallState::Falling(_) => {
            if !matches!(*primary_state, PrimaryState::Penalized) {
                //     if self.prev_behavior_for_standup.is_none() {
                //         self.prev_behavior_for_standup = Some(self.behavior.clone());
                //     }
                //     self.behavior = BehaviorState::CatchFall(CatchFall);
            }
            return;
        }
        FallState::None => {
            if matches!(behavior, BehaviorState::CatchFall) {
                // self.behavior = self.prev_behavior_for_standup.take().unwrap();
                return;
            }
        }
    }

    if let Some(message) = game_controller_message {
        if message.game_phase == GamePhase::PenaltyShoot {
            if message.kicking_team == 8 {
                commands.set_role(Striker::WalkWithBall);
            } else {
                commands.set_behavior(Stand);
                return;
            }
        }
    }

    // Change this to a system, also in Stiker
    let most_confident_ball = bottom_balls
        .most_confident_ball()
        .map(|b| b.position)
        .or(top_balls.most_confident_ball().map(|b| b.position));

    let ball_or_origin = most_confident_ball.unwrap_or(Point2::origin());

    match *primary_state {
        PrimaryState::Sitting => commands.set_behavior(Sitting),
        PrimaryState::Standby
        | PrimaryState::Penalized
        | PrimaryState::Finished
        | PrimaryState::Calibration => commands.set_behavior(Stand),
        PrimaryState::Initial => commands.set_behavior(StandLookAt {
            target: Point2::origin(),
        }),
        PrimaryState::Ready => commands.set_behavior(WalkToSet {
                // Replaced with check in the behavior
            }),
        PrimaryState::Set => commands.set_behavior(StandLookAt {
            target: ball_or_origin,
        }),
        PrimaryState::Playing { .. } => {}
    }
}
