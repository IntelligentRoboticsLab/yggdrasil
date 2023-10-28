// TODO: put in RFC
//
// Thoughts:
//
// Right now we would need to pass all information necessary for every possible behaviour into the
// execute function of each behaviour implemenation, which is not ideal. Especially when behaviour
// modules are statefull and need to keep track
// of their own state this might become a little chaotic and makes it unpleasant to add new
// behaviours that would depend on new states. We could probably wrap all the data in a single
// struct to make this less cumbersome.
//
// An alternative might be defining behaviours as systems so they can specify the data they
// need. This means every behaviour would run every cycle. We would then only use the results
// we actually need. Will make it a lot easier to add new behaviours with their own states.
// However, everything is then always run which is a bit of a waist of computation. If we
// decide that this computation is not super expensive then this would probably be the best option.
//
//
// Something that is not entirely clear for me yet is how we would incorporate for example the
// walking engine. Currently the walking engine directly writes to the nao control message. Maybe
// the walking engine could be dependent on the Behaviour state for computing the correct joint
// values then store the resulting joint values in some intermediate representation which is then
// used by the behaviour modules. Lets say that in a behaviour we want to walk to a specific position, how would we pass that
// information to the walking engine in this case?
//
// A solution might be one similar to what hulks has, the result of a behaviour is a motion
// command. Which is then used by other modules for computing the correct joint angles.

// Instead of hashmap, use a match statement without dymanic dispatch
// Add context
// Add Behaviour states

use std::error::Error;

use crate::{
    behaviour::Role,
    behaviour::primary_state::PrimaryState,
    game_phase::GamePhase,
};

use miette::{Diagnostic, Result};
use nidhogg::NaoControlMessage;
use tyr::prelude::*;

use crate::behaviour::behaviour_engine::behaviours::*;

#[derive(Debug, Eq, PartialEq, Hash, Clone, Copy)]
pub enum BehaviourType {
    Defend,
    Dribble,
    Falling,
    GetReadyForKickOff,
    GetReadyForPenalty,
    LookForBall,
    LookAround(LookAroundState),
    None,
    Penalized,
    Stand,
    Sit,
    Unstiff,
    WalkToBall,
}

// type Behaviour = Box<dyn ImplBehaviour + Sync + Send>;

pub trait ImplBehaviour {
    fn execute(&self, ctx: &mut BehaviourContext) -> NaoControlMessage;
}

pub struct BehaviourEngine {
    current_behaviour: BehaviourType,
}

// struct BehaviourContext;
#[derive(Debug, Clone, Copy)]
pub struct BehaviourContext {
    pub ball: [f64; 3],
    pub game_phase: GamePhase,
    pub current_behaviour: BehaviourType,
    pub robot_pos: [f64; 3],
    pub player_number: i32,
    pub primary_state: PrimaryState,
    pub role: Role,
}

impl BehaviourEngine {
    pub fn new() -> Self {
        BehaviourEngine {
            current_behaviour: BehaviourType::None
        }
    }

    pub fn execute_current_behaviour(&self, ctx: &mut BehaviourContext) -> Result<NaoControlMessage, i32> {
        match self.current_behaviour {
            BehaviourType::LookAround(_) => Ok(LookAround.execute(ctx)),
            _ => Err(0),
        }
    }

    pub fn transition(&mut self, behaviour_type: BehaviourType) {
        self.current_behaviour = behaviour_type;
    }
}

#[system]
pub fn executor(
    engine: &mut BehaviourEngine,
    ctrl_msg: &mut NaoControlMessage,
    //ball_positio
    // ball_position: Vec<f32>,
    // robot_position: Vec<32>,
    behaviour_context: &mut BehaviourContext, // Pass all information needed to the engine
) -> Result<()> {
    let mut ctx = *behaviour_context;

    // pass to this instad of
    let message = match engine.execute_current_behaviour(&mut ctx) {
        Err(_e) => NaoControlMessage::default(),
        Ok(res) => res,
    };

    *ctrl_msg = message;

    // ctrl_msg.position = position;
        // ctrl_msg.stiffness = stiffness;
        // ctrl_msg.sonar = sonar;
        // ctrl_msg.left_ear = left_ear;
        // ctrl_msg.right_ear = right_ear;
    // ctrl_msg.chest = chest;
        // ctrl_msg.left_eye = left_eye;
        // ctrl_msg.right_eye = right_eye;
        // ctrl_msg.left_foot = left_foot;
        // ctrl_msg.right_foot = right_foot;
        // ctrl_msg.skull = skull;

    Ok(())
}

// Do not yet 100% understand how I should return errors
#[derive(Debug)]
struct NoImplementationError;
impl std::fmt::Display for NoImplementationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Could not find implementation")
    }
}
impl Error for NoImplementationError {}
impl Diagnostic for NoImplementationError {}
