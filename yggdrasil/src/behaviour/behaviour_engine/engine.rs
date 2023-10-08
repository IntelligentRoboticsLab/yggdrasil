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
    fn execute(&self) -> NaoControlMessage;
}

pub struct BehaviourEngine {
    current_behaviour: BehaviourType,
    behaviours: HashMap<BehaviourType, Behaviour>,
}

#[derive(Debug)]
struct NoImplementationError;
impl std::fmt::Display for NoImplementationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Could not find implementation")
    }
}
impl Error for NoImplementationError {}
impl Diagnostic for NoImplementationError {}

impl BehaviourEngine {
    pub fn new() -> Self {
        BehaviourEngine {
            current_behaviour: BehaviourType::None,
            behaviours: HashMap::new(),
        }
    }

    pub fn execute_current_behaviour(&self) -> Result<()> {
        if let Some(behaviour_implementation) = self.behaviours.get(&self.current_behaviour) {
            behaviour_implementation.execute();
        } else {
            return Err(Report::new(NoImplementationError {}));
        }

        Ok(())
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
pub fn executor(engine: &mut BehaviourEngine) -> Result<()> {
    engine.execute_current_behaviour()?;
    Ok(())
}
