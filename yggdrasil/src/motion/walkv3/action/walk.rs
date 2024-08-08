use std::time::Duration;

use nidhogg::types::{FillExt, LegJoints};

use crate::{
    kinematics::{self, FootOffset},
    motion::{
        walk::engine::Side,
        walkv3::{
            feet::Feet,
            step::{PlannedStep, StepRequest},
            step_state::StepState,
        },
    },
    nao::manager::{NaoManager, Priority},
};

use super::{UpdateContext, WalkAction};

pub struct Walking {
    pub step_state: Option<StepState>,
    pub requested_step: StepRequest,
    pub side: Side,
}

impl Walking {
    pub fn new() -> Self {
        Self {
            step_state: None,
            side: Side::Left,
            requested_step: StepRequest {
                forward: 0.0,
                left: 0.04,
                turn: 1.5,
            },
        }
    }
}

impl WalkAction for Walking {
    fn update(&mut self, ctx: &UpdateContext) {
        if self.step_state.is_none() {
            let mut state = StepState::default();
            state.planned_step =
                PlannedStep::from_request(&ctx.kinematics, self.requested_step, self.side);

            self.step_state = Some(state);
        }

        if let Some(step_state) = &mut self.step_state {
            step_state.update(ctx.delta_time);

            if step_state.duration > step_state.planned_step.duration {
                self.side = self.side.next();

                let mut state = StepState::default();
                state.planned_step =
                    PlannedStep::from_request(&ctx.kinematics, self.requested_step, self.side);

                self.step_state = Some(state);
            }
        }
    }

    fn apply(&self, nao: &mut NaoManager) {
        let Some(state) = &self.step_state else {
            return;
        };

        let feet = state.compute_feet();

        let (left, right) = match self.side {
            Side::Left => (feet.swing, feet.support),
            Side::Right => (feet.support, feet.swing),
        };

        let left = FootOffset::from_isometry(left, 0.180);
        let right = FootOffset::from_isometry(right, 0.180);

        // tracing::info!("left: {:?}, right: {:?}", left, right);

        let (left_leg, right_leg) = kinematics::inverse::leg_angles(&left, &right);

        nao.set_legs(
            LegJoints::builder()
                .left_leg(left_leg)
                .right_leg(right_leg)
                .build(),
            LegJoints::fill(0.7),
            Priority::Critical,
        );
    }
}
