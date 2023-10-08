use std::{collections::HashMap, error::Error};

use miette::{Diagnostic, Report, Result};
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
    LookAround,
    None,
    Penalized,
    Stand,
    Sit,
    Unstiff,
    WalkToBall,
}

type Behaviour = Box<dyn ImplBehaviour + Sync + Send>;

pub trait ImplBehaviour {
    // Here we need to pass all information necessary for every possible behaviour.
    // An alternative might be defining behaviours as systems so they can specify the data they
    // need. However, in that case all those modules would be run every cycle which is a waist.
    fn execute(&self) -> NaoControlMessage;
}

pub struct BehaviourEngine {
    current_behaviour: BehaviourType,
    behaviours: HashMap<BehaviourType, Behaviour>,
}

impl BehaviourEngine {
    pub fn new() -> Self {
        BehaviourEngine {
            current_behaviour: BehaviourType::None,
            behaviours: HashMap::new(),
        }
    }

    pub fn execute_current_behaviour(&self) -> Result<NaoControlMessage> {
        if let Some(behaviour_implementation) = self.behaviours.get(&self.current_behaviour) {
            Ok(behaviour_implementation.execute())
        } else {
            Err(Report::new(NoImplementationError {}))
        }
    }

    pub fn transition(&mut self, behaviour_type: BehaviourType) {
        self.current_behaviour = behaviour_type;
    }

    pub fn add_behaviour(&mut self, new_behaviour: BehaviourType, implementation: Behaviour) {
        self.behaviours.insert(new_behaviour, implementation);
    }
}

pub fn initializer(storage: &mut Storage) -> Result<()> {
    let mut behaviour_engine = BehaviourEngine::new();

    // Add more behaviours here.
    behaviour_engine.add_behaviour(BehaviourType::LookAround, Box::new(LookAround {}));

    storage.add_resource(Resource::new(behaviour_engine))?;
    Ok(())
}

#[system]
pub fn executor(engine: &mut BehaviourEngine, ctrl_msg: &mut NaoControlMessage) -> Result<()> {
    let NaoControlMessage {
        position,
        stiffness,
        sonar,
        left_ear,
        right_ear,
        chest,
        left_eye,
        right_eye,
        left_foot,
        right_foot,
        skull,
    } = engine.execute_current_behaviour()?;

    // This can be overwritten over override field written by other modules,
    // prob needs improvement
    ctrl_msg.position = position;
    ctrl_msg.stiffness = stiffness;
    ctrl_msg.sonar = sonar;
    ctrl_msg.left_ear = left_ear;
    ctrl_msg.right_ear = right_ear;
    ctrl_msg.chest = chest;
    ctrl_msg.left_eye = left_eye;
    ctrl_msg.right_eye = right_eye;
    ctrl_msg.left_foot = left_foot;
    ctrl_msg.right_foot = right_foot;
    ctrl_msg.skull = skull;

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
