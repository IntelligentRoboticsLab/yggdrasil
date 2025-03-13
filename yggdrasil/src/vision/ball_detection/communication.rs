use nalgebra as na;

pub struct CommunicatedBalls {
		/// For keeping track what position we've sent out.
    sent: Option<na::Point2<f32>>,
    /// For keeping track what positions we've received.
    received: HashMap<SocketAddr, Option<na::Point2<f32>>>,
}

impl CommunicatedBalls {
    fn should_update(&mut self, ball: &Option<na::Point2>) -> bool {
        todo!("check if `self.sent` and `ball` are different (enough)")
    }
}

fn communicate_balls(
    mut tc: ResMut<TeamCommunication>,
    mut communicated_balls: ResMut<CommunicatedBalls>,
    top_balls: Res<Balls<Top>>,
    bottom_balls: Res<Balls<Bottom>>,
) {
    let most_confident_ball = bottom_balls
        .most_confident_ball()
        .map(|b| b.position)
        .or(top_balls.most_confident_ball().map(|b| b.position));

    if communicated_balls.should_update(&most_confident_ball) {
        tc.outbound_mut().update_or_push(TeamMessage::Ball(most_confident_ball));
        communicated_balls.sent = most_confident_ball;
    }

    while let Some((_when, sender, ball) = todo!("receive updates using `take_map`") {
        communicated_balls.received[sender] = ball;
    }
}