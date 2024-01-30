use nidhogg::types::{FillExt, JointArray, LeftLegJoints, RightLegJoints};

use super::{WalkContext, WalkState, WalkStateKind};

/// The hip height of the robot when sitting, 10cm
const _SITTING_HIP_HEIGHT: f32 = 0.0975;

pub(crate) struct IdleState;

impl WalkState for IdleState {
    fn next_state<'a>(&self, context: &'a mut WalkContext) -> WalkStateKind {
        context.control_message.stiffness = JointArray::<f32>::builder()
            .left_leg_joints(LeftLegJoints::fill(0.0))
            .right_leg_joints(RightLegJoints::fill(0.0))
            .build();

        WalkStateKind::Idle(IdleState)
    }
}
