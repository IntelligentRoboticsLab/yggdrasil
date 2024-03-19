use crate::behavior::engine::{Behavior, Context};
use nidhogg::NaoControlMessage;

/// During a match the chest button is pressed before starting a match.
/// Once this is done, the robots are placed at the edge of the field from
/// which they will walk to their `Ready` positions.
///
/// This is the behaviour of the robot once the chest button is pressed.
/// In this state the robot will stand up straight and look at the middle
/// circle to make it easier to place the robot in the correct position.
/// After being placed at the side of the field the robot looks around to
/// improve localisation.
#[derive(Copy, Clone, Debug, Default)]
pub struct Initial;

impl Behavior for Initial {
    fn execute(&mut self, context: Context, _control_message: &mut NaoControlMessage) {
        println!("test");
        // TODO
        // - Add field config
        // - Add robot starting position for each player number
        // - Load current robot playing number
        //
        // - Stand up straight
        // - Look at middle circle
        // - Once standing turn head back and fourth 3x after pressing of chest button
        println!("{}", context.layout_config.field.width);
        println!("{}", context.layout_config.field.length);
    }
}
