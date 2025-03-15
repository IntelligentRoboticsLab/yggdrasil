use std::{collections::HashMap, net::SocketAddr};

use bevy::prelude::*;
use nalgebra as na;

use crate::communication::{TeamCommunication, TeamMessage};

// Import camera proposals
use super::classifier::Balls;

// Constant for the minimum acceptable change
const MIN_CHANGE: f32 = 0.1;

pub struct CommunicatedBalls {
    /// For keeping track what position we've sent out.
    sent: Option<na::Point2<f32>>,
    /// For keeping track what positions we've received.
    received: HashMap<SocketAddr, Option<na::Point2<f32>>>,
}

impl CommunicatedBalls {
    /// Check it the position has changed enough from last frame.
    fn change_enough(&mut self, ball: &Option<na::Point2<f32>>) -> bool {
        match (ball, &self.sent) {
            (None, None) => false,
            (None, Some(_)) => true,
            (Some(_), None) => true,
            (Some(old), Some(new)) => na::distance(old, new) > MIN_CHANGE,
        }
    }

    /// Send your ball position (even if it's None) as a message.
    fn send_message(&mut self, ball_position: Option<na::Point2<f32>>, tc: &mut TeamCommunication){
        tc.outbound_mut()
            .update_or_push(TeamMessage::DetectedBall(ball_position))
            .expect("Unable to encode detected ball");
        self.sent = ball_position;
    }

    /// Receive messages.
    // 2.A.a. If no other robot are detecting a ball, we return the same None we had
    // 2.A.b. If there are other robots detecting a ball, we take one from theirs as our own.
    fn receive_messages(comms: &mut TeamCommunication) -> Option<na::Point2<f32>> {
        let mut received_ball = None;

        while let Some((_, _, ball)) = comms.inbound_mut().take_map(|_, _, what| match what {
            TeamMessage::DetectedBall(ball) => Some(*ball),
            _ => None,
        }) {
            received_ball = received_ball.or(ball);
        }

        received_ball
    }

    fn communicate_balls(
        &mut self,
        tc: &mut TeamCommunication,
        most_confident_ball: Option<na::Point2<f32>>,
    ) -> Option<na::Point2<f32>> {
        // 1. Check if it has changed enough and if so, we send a message.
        if self.change_enough(&most_confident_ball) {
            self.send_message(most_confident_ball, tc)
        }

        // 2. Receive messages only if our current ball is None.
        if let ball @ Some(_) = most_confident_ball {
            ball
        } else {
            Self::receive_messages(tc)
        }
    }
}

fn communicate_balls(
    mut tc: ResMut<TeamCommunication>,
    mut communicated_balls: ResMut<CommunicatedBalls>,
    top_balls: Res<Balls<Top>>,
    bottom_balls: Res<Balls<Bottom>>,
) {
    let ball = bottom_balls
        .most_confident_ball()
        .map(|b| (b.timestamp, b.robot_to_ball))
        .or(top_balls
            .most_confident_ball()
            .map(|b| (b.timestamp, b.robot_to_ball)));

    todo!("marinita pls finish спс)))")
}
