use bevy::prelude::*;
use nalgebra::{self as na, Point2};

use crate::{
    communication::{TeamCommunication, TeamMessage},
    core::debug::DebugContext,
    localization::RobotPose,
    nao::Cycle,
};

// Import camera proposals
use super::ball_tracker::BallTracker;

// Constant for the minimum acceptable change
const MIN_CHANGE: f32 = 0.1;

pub struct CommunicatedBallsPlugin;

#[derive(Resource, Default, Debug)]
pub struct LastReceivedBall {
    pub position: Option<Point2<f32>>,
}

impl Plugin for CommunicatedBallsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TeamBallPosition>()
            .init_resource::<CommunicatedBalls>()
            .init_resource::<LastReceivedBall>()
            .add_systems(Update, communicate_balls_system);
    }
}

#[derive(Resource, Default, Debug)]
pub struct TeamBallPosition(pub Option<Point2<f32>>);

#[derive(Resource, Debug, Default)]
pub struct CommunicatedBalls {
    /// For keeping track what position we've sent out.
    sent: Option<na::Point2<f32>>,
}

impl CommunicatedBalls {
    /// Check it the position has changed enough from last frame.
    fn change_enough(&mut self, ball: Option<&na::Point2<f32>>) -> bool {
        match (ball, &self.sent) {
            (None, None) => false,
            (None, Some(_)) => true,
            (Some(_), None) => true,
            (Some(old), Some(new)) => na::distance(old, new) > MIN_CHANGE,
        }
    }

    /// Send your ball position (even if it's None) as a message.
    fn send_message(&mut self, ball_position: Option<na::Point2<f32>>, tc: &mut TeamCommunication) {
        tc.outbound_mut()
            .update_or_push(TeamMessage::DetectedBall(ball_position))
            .expect("Unable to encode detected ball");
        self.sent = ball_position;
    }

    /// Receive messages.
    // 2.A.a. If no other robot are detecting a ball, we return the same None we had
    // 2.A.b. If there are other robots detecting a ball, we take one from theirs as our own.
    fn receive_messages(
        comms: &mut TeamCommunication,
        pose: &RobotPose,
    ) -> Option<na::Point2<f32>> {
        let mut received_ball = None;

        while let Some((_, _, ball)) = comms.inbound_mut().take_map(|_, _, what| match what {
            TeamMessage::DetectedBall(ball) => Some(*ball),
            _ => None,
        }) {
            received_ball = received_ball.or(ball);
        }
        // If we received a ball, transform it from world coordinates to robot coordinates
        received_ball.map(|ball| pose.world_to_robot(&ball))
    }
}

#[allow(clippy::too_many_arguments)]
fn communicate_balls_system(
    mut communicated_balls: ResMut<CommunicatedBalls>,
    mut tc: ResMut<TeamCommunication>,
    ball_tracker: Res<BallTracker>,
    mut team_ball_position: ResMut<TeamBallPosition>,
    pose: Res<RobotPose>,
    mut last_received: ResMut<LastReceivedBall>,
    ctx: DebugContext,
    cycle: Res<Cycle>,
) {
    let optional_ball_position = ball_tracker.stationary_ball();

    // 1. Check if it has changed enough and if so, we send a message.
    // let optional_ball_position = ball_position.map(|ball_position| ball_position.0);
    if communicated_balls.change_enough(optional_ball_position.as_ref()) {
        let transformed_position = optional_ball_position.map(|pos| pose.robot_to_world(&pos));
        communicated_balls.send_message(transformed_position, &mut tc);
    }

    if let Some(new_pos) = CommunicatedBalls::receive_messages(&mut tc, &pose) {
        last_received.position = Some(new_pos);
    } else {
        last_received.position = None;
    }

    team_ball_position.0 = optional_ball_position.or_else(|| last_received.position);

    if let Some(pos) = team_ball_position.0 {
        let global = pose.robot_to_world(&pos);
        ctx.log_with_cycle(
            "/team_ball",
            *cycle,
            &rerun::Points3D::new([(global.x, global.y, 0.01)])
                .with_radii([0.1])
                .with_labels(["team_ball"]),
        );
    } else {
        ctx.log_with_cycle("/team_ball", *cycle, &rerun::Clear::recursive());
    }
}
