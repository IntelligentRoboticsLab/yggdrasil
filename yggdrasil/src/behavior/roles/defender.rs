use bevy::prelude::*;
use heimdall::{Bottom, Top};

use crate::{
    behavior::{
        behaviors::RlDefenderSearchBehavior,
        engine::{in_role, CommandsBehaviorExt, RoleState, Roles},
        roles::Striker,
    },
    vision::ball_detection::classifier::Balls,
};

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

pub fn defender_role(
    mut commands: Commands,
    top_balls: Res<Balls<Top>>,
    bottom_balls: Res<Balls<Bottom>>,
) {
    let most_confident_ball = bottom_balls
        .most_confident_ball()
        .map(|b| b.position)
        .or(top_balls.most_confident_ball().map(|b| b.position));

    if most_confident_ball.is_none() {
        commands.set_behavior(RlDefenderSearchBehavior);
    } else {
        commands.set_role(Striker::WalkToBall);
    }
}
