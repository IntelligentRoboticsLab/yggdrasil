use nidhogg::types::{FillExt, LegJoints};

use crate::{
    kinematics::{self, FootOffset},
    nao::manager::{NaoManager, Priority},
};

use super::{UpdateContext, WalkAction};

const STANDING_HIP_HEIGHT: f32 = 0.180;

pub struct Standing {
    pub current_hip_height: f32,
}

impl Standing {
    pub fn with_hip_height(current_hip_height: f32) -> Self {
        Self { current_hip_height }
    }
}

impl WalkAction for Standing {
    fn update(&mut self, _ctx: &UpdateContext) {
        self.current_hip_height = (self.current_hip_height + 0.002).min(STANDING_HIP_HEIGHT);
    }

    fn apply(&self, nao: &mut NaoManager) {
        let zero = FootOffset::zero(self.current_hip_height);

        let (left_leg, right_leg) = kinematics::inverse::leg_angles(&zero, &zero);

        nao.set_legs(
            LegJoints::builder()
                .left_leg(left_leg)
                .right_leg(right_leg)
                .build(),
            LegJoints::fill(0.8),
            Priority::Critical,
        );
    }
}
