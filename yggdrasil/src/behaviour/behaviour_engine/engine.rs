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

#[derive(Eq, PartialEq, Hash)]
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
    fn execute(&self) -> NaoControlMessage;
}

pub struct BehaviourEngine {
    current_behaviour: BehaviourType,
    current_behaviour_context: BehaviourContext
}

// struct BehaviourContext;
pub struct BehaviourContext {
    pub ball: Vec<f32>,
    pub game_phase: GamePhase,
    pub robot_pos: Vec<f32>,
    pub player_number: i32,
    pub primary_state: PrimaryState,
    pub role: Role,
}

impl BehaviourEngine {
    pub fn new() -> Self {
        BehaviourEngine {
            current_behaviour: BehaviourType::None,
        }
    }

    pub fn execute_current_behaviour(&self, &mut ctx) -> Result<NaoControlMessage> {


        Ok(match self.current_behaviour {
            BehaviourType::LookAround(_) => LookAround.execute(ctx),
            _ => NaoControlMessage::default(),
        })
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
) -> Result<()> {
    let mut ctx = BehaviourContext {
    // &mut engin.current_behaviour
    // ball positio
    };


    // let NaoControlMessage {
    //     position,
    //     stiffness,
    //     sonar,
    //     left_ear,
    //     right_ear,
    //     chest,
    //     left_eye,
    //     right_eye,
    //     left_foot,
    //     right_foot,
    //     skull,
    // }
    // pass to this instad of 
    ctrl_msg = engine.execute_current_behaviour(&mut ctx)?;

    // This can be overwritten over override field written by other modules,
    // prob needs improvement
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
