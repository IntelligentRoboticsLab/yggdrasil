use nidhogg::types::{FillExt, LegJoints};

use crate::nao::manager::{NaoManager, Priority};

use super::{UpdateContext, WalkAction};

const SITTING_HIP_HEIGHT: f32 = 0.0975;

pub struct Sitting {
    pub current_hip_height: f32,
}

impl Sitting {
    pub fn with_hip_height(current_hip_height: f32) -> Self {
        Self { current_hip_height }
    }
}

impl WalkAction for Sitting {
    fn update(&mut self, _ctx: &UpdateContext) {
        self.current_hip_height = (self.current_hip_height - 0.002).max(SITTING_HIP_HEIGHT);
    }

    fn apply(&self, nao: &mut NaoManager) {
        let foot = crate::kinematics::FootOffset::zero(self.current_hip_height);
        let (left_leg, right_leg) = crate::kinematics::inverse::leg_angles(&foot, &foot);

        nao.set_legs(
            LegJoints::builder()
                .left_leg(left_leg)
                .right_leg(right_leg)
                .build(),
            LegJoints::fill(0.2),
            Priority::Critical,
        );
    }
}
