use bevy::ecs::system::ResMut;
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
    fn send_message(&mut self, ball_position: Option<na::Point2<f32>>) {
        tc.outbound_mut()
            .update_or_push(TeamMessage::Ball(ball_position));
        self.sent = ball_position;
    }

    /// Receive messages.
    // 2.A.a. If no other robot are detecting a ball, we return the same None we had
    // 2.A.b. If there are other robots detecting a ball, we take one from theirs as our own.
    fn receive_messages(comun: &mut TeamCommunication) {
        let mut received_ball = None;
        while let Some((_, who, ball)) = comms.inbound_mut().take_map(|_, _, what| match what {
            TeamMessage::DetectedBall(ball) => Some(ball),
            _ => None,
        }) {
            received_ball = received_ball.or(ball);
        }
        received_ball
    }

    fn communicate_balls(
        mut tc: ResMut<TeamCommunication>,
        mut most_confident_ball: Option<na::Point2<f32>>,
    ) {
        // 1. Check if it has changed enough and if so, we send a message.
        let has_changed =
            CommunicatedBalls::changed_enough(most_confident_ball, comunicated_balls.sent);
        if has_changed {
            self.send_message(most_confident_ball)
        }

        // 2. Receive messages only if our current ball is None.
        // 2.A. If its None we check the received messages.
        if most_confident_ball.is_none() {
            let ball = receive_messages(TeamCommunication);
        // 2.B. If our current ball is not None, then we just use our ball.
        } else {
            ball = most_confident_ball
        }

        // 3. We return the ball we are using
        ball
    }
}
