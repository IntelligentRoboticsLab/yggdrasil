// use bevy::prelude::*;
// use heimdall::{Bottom, Top};

// use crate::{
//     behavior::{
//         behaviors::Observe,
//         engine::{in_role, BehaviorState, CommandsBehaviorExt},
//         primary_state::update_primary_state,
//     },
//     vision::ball_detection::classifier::Balls,
// };

// use crate::behavior::engine::{Role, Roles};

// use super::Striker;

// /// Plugin for the Instinct role
// pub struct InstinctRolePlugin;

// impl Plugin for InstinctRolePlugin {
//     fn build(&self, app: &mut App) {
//         app.add_systems(
//             Update,
//             behavior
//                 .after(update_primary_state)
//                 .run_if(in_role::<Instinct>),
//         );
//     }
// }

// /// The [`Instinct`] role is a no-role state.
// #[derive(Resource)]
// pub struct Instinct;
// impl Roles for Instinct {
//     const STATE: Role = Role::Instinct;
// }

// pub fn behavior(
//     mut commands: Commands,
//     top_balls: Res<Balls<Top>>,
//     bottom_balls: Res<Balls<Bottom>>,
//     behavior_state: Res<State<BehaviorState>>,
// ) {
//     let most_confident_ball = bottom_balls
//         .most_confident_ball()
//         .map(|b| b.position)
//         .or(top_balls.most_confident_ball().map(|b| b.position));

//     if most_confident_ball.is_some() {
//         commands.set_role(Striker::WalkToBall);
//     } else if behavior_state.get() != &BehaviorState::Observe {
//         commands.set_behavior(Observe::default());
//     }
// }
