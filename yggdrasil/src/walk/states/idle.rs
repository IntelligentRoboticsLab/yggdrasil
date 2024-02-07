use nidhogg::types::{FillExt, JointArray, LeftLegJoints, RightLegJoints};

use crate::{
    kinematics::{self, FootOffset},
    walk::WalkingEngineConfig,
};

use super::{WalkContext, WalkState, WalkStateKind};

#[derive(Debug)]
pub(crate) struct IdleState {
    pub(crate) hip_height: f32,
}

impl IdleState {
    pub fn new(config: &WalkingEngineConfig) -> Self {
        Self {
            hip_height: config.hip_height,
        }
    }
}

impl WalkState for IdleState {
    fn next_state(&self, context: WalkContext) -> WalkStateKind {
        let hip_height = self.hip_height;
        let foot_position = FootOffset {
            forward: 0.0,
            left: 0.0,
            turn: 0.0,
            hip_height,
            lift: 0.0,
        };

        let (left_leg, right_leg) = kinematics::inverse::leg_angles(&foot_position, &foot_position);
        context.control_message.position = JointArray::<f32>::builder()
            .left_leg_joints(left_leg)
            .right_leg_joints(right_leg)
            .build();
        context.control_message.stiffness = JointArray::<f32>::builder()
            .left_leg_joints(LeftLegJoints::fill(0.5))
            .right_leg_joints(RightLegJoints::fill(0.5))
            .build();

        // Slowly stand up, by moving towards the idle hip height.
        WalkStateKind::Idle(IdleState {
            hip_height: (hip_height + 0.0025).min(context.config.hip_height),
        })
    }
}
