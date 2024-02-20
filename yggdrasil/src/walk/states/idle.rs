use crate::{
    kinematics::FootOffset,
    walk::WalkingEngineConfig,
};

use super::{WalkContext, WalkState, WalkStateKind};

#[derive(Debug, Clone)]
pub struct IdleState {
    pub hip_height: f32,
}

impl IdleState {
    pub fn new(config: &WalkingEngineConfig) -> Self {
        Self {
            hip_height: config.sitting_hip_height,
        }
    }
}

impl WalkState for IdleState {
    fn next_state(self, context: WalkContext) -> WalkStateKind {
        // Slowly stand up, by moving towards the idle hip height.
        WalkStateKind::Idle(IdleState {
            hip_height: ( self.hip_height + 0.0025).min(context.config.hip_height),
        })
    }

    fn get_foot_offsets(&self) -> (FootOffset, FootOffset) {
        let hip_height = self.hip_height;
        let foot_position = FootOffset {
            forward: 0.0,
            left: 0.0,
            turn: 0.0,
            hip_height,
            lift: 0.0,
        };

        (foot_position, foot_position)
    }
}
