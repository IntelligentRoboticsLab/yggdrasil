use nalgebra as na;

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
        match(ball, &self.sent){
            (None, None) -> false,
            (None, Some(_)) -> true,
            (Some(_), None) -> true,
            (Some(old ), Some(new)) -> na::distance(old, new) > MIN_CHANGE,
        }

    /// Send your ball position (even if it's None) as a message.
    fn send_message(ball_position, ){
        // Placeholder for sending a message
        }
    }

fn communicate_balls(
    mut tc: ResMut<TeamCommunication>,
    mut communicated_balls: ResMut<CommunicatedBalls>,
    top_balls: Res<Balls<Top>>,
    bottom_balls: Res<Balls<Bottom>>,
    ) {
    // 1. Receive the ball
    // Here we take the most confident ball from the top and bottom 
    // cameras, or None if there are no balls.
    let most_confident_ball = bottom_balls
        .most_confident_ball()
        .map(|b| b.position)
        .or(top_balls.most_confident_ball().map(|b| b.position));

    // 2. Check if it has changed enough
    let has_changed = CommunicatedBalls::changed_enough(most_confident_ball, comunicated_balls.sent);
    
    // 3. Check if the current ball is None
    match most_confident_ball {
        // 3.A. If it was None
        None => {
            let messages = use_received_messages();
            
            if has_changed {
                send_message();
            }
        }
        // 3.B. If it was not None
        Some(_) => {
            if has_changed {
                send_message();
                }
            }
        }
    }