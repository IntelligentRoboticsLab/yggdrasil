use bevy::prelude::*;
use bifrost::communication::{GameControllerMessage, GamePhase};
use heimdall::{Bottom, Top};
use nalgebra::Point2;

use crate::{
    behavior::{
        behaviors::{CatchFall, Sitting, Stand, StandLookAt, Standup, WalkToSet},
        engine::CommandsBehaviorExt,
        primary_state::{update_primary_state, PrimaryState},
        roles::Striker,
    },
    core::config::showtime::PlayerConfig,
    motion::walk::engine::WalkingEngine,
    sensor::{button::HeadButtons, falling::FallState},
    vision::ball_detection::classifier::Balls,
};

use crate::behavior::engine::{BehaviorState, Role, Roles};

/// Plugin for the Instinct role
pub struct InstinctRolePlugin;

impl Plugin for InstinctRolePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            behavior.after(update_primary_state), // .run_if(resource_exists::<Instinct>),
        );
    }
}

/// The [`Instinct`] role is a no-role state.
#[derive(Resource)]
pub struct Instinct;
impl Roles for Instinct {
    const STATE: Role = Role::Instinct;
}

#[allow(clippy::too_many_arguments)]
pub fn behavior(
    mut commands: Commands,
    state: Res<State<BehaviorState>>,
    walking_engine: Res<WalkingEngine>,
    head_buttons: Res<HeadButtons>,
    primary_state: Res<PrimaryState>,
    game_controller_message: Option<Res<GameControllerMessage>>,
    fall_state: Res<FallState>,
    top_balls: Res<Balls<Top>>,
    bottom_balls: Res<Balls<Bottom>>,
    standup_state: Option<Res<Standup>>,
    player_config: Res<PlayerConfig>,
) {
    commands.set_role(Instinct);

    let behavior = state.get();

    if behavior == &BehaviorState::StartUp {
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

    if standup_state.is_some_and(|s| !s.completed()) {
        return;
    }

    // next up, damage prevention and standup motion takes precedence
    match fall_state.as_ref() {
        FallState::Lying(_) => {
            commands.set_behavior(Standup::default());

            return;
        }
        FallState::Falling(_) => {
            if !matches!(*primary_state, PrimaryState::Penalized) {
                commands.set_behavior(CatchFall);
            }
            return;
        }
        FallState::None => {}
    }

    if let Some(message) = game_controller_message {
        if message.game_phase == GamePhase::PenaltyShoot {
            if message.kicking_team == player_config.team_number {
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
        PrimaryState::Ready => commands.set_behavior(WalkToSet {}),
        PrimaryState::Set => commands.set_behavior(StandLookAt {
            target: ball_or_origin,
        }),
        PrimaryState::Playing { .. } => {
            decide_role(commands, player_config, bottom_balls, top_balls);
        }
    }
}

fn decide_role(
    commands: Commands,
    player_config: Res<PlayerConfig>,
    bottom_balls: Res<Balls<Bottom>>,
    top_balls: Res<Balls<Top>>,
) {
    // Change this to a system, also in Stiker
    let most_confident_ball = bottom_balls
        .most_confident_ball()
        .map(|b| b.position)
        .or(top_balls.most_confident_ball().map(|b| b.position));

    // Only here should we activate the role deciding behavior
    Role::assign_role(
        commands,
        most_confident_ball.is_some(),
        player_config.player_number,
    );
}
