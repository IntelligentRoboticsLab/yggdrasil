use std::collections::HashMap;

use miette::Result;
use tyr::prelude::*;

#[derive(Eq, PartialEq, Hash)]
pub enum Behaviour {
    Defend,
    Dribble,
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

pub struct BehaviourEngine {
    current_behaviour: Behaviour,
    behaviours: HashMap<Behaviour, i32>,
}

impl BehaviourEngine {
    pub fn new() -> Self {
        BehaviourEngine {
            current_behaviour: Behaviour::None,
            behaviours: HashMap::new(),
        }
    }

    pub fn transition(&mut self, new_behaviour: Behaviour) {
        self.current_behaviour = new_behaviour;
    }

    pub fn add_behaviour(&mut self, new_behaviour: Behaviour, t: i32) {
        self.behaviours.insert(new_behaviour, t);
    }
}

pub fn initializer(storage: &mut Storage) -> Result<()> {
    let mut behaviour_engine = BehaviourEngine::new();

    // Add more behaviours here.
    behaviour_engine.add_behaviour(Behaviour::Sit, 4);

    storage.add_resource(Resource::new(behaviour_engine))?;
    Ok(())
}

#[system]
pub fn executor(engine: &mut BehaviourEngine) -> Result<()> {
    // engine.current_behaviour.execute();
    Ok(())
}
