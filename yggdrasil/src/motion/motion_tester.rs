use crate::{
    filter::{
        button::HeadButtons,
        falling::{Fall, FallState, LyingDirection},
    },
    nao::manager::{NaoManager, Priority},
};
use miette::Result;
use nidhogg::{
    types::{FillExt, JointArray},
    NaoState,
};

use tyr::prelude::*;

use crate::motion::motion_manager::MotionManager;
use crate::motion::motion_types::MotionType;

pub struct MotionTester;

impl Module for MotionTester {
    fn initialize(self, app: App) -> Result<App> {
        Ok(app.add_system(debug_testmotion))
    }
}

#[system]
fn debug_testmotion(
    head_button: &mut HeadButtons,
    mmng: &mut MotionManager,
    nao_state: &NaoState,
    nao_manager: &mut NaoManager,
    fall: &Fall,
) -> Result<()> {
    if head_button.middle.is_tapped() {
        match fall.state {
            FallState::Lying(LyingDirection::FacingDown) => {
                mmng.start_new_motion(MotionType::StandupStomach, Priority::High)
            }
            FallState::Lying(LyingDirection::FacingUp) => {
                mmng.start_new_motion(MotionType::StandupBack, Priority::High)
            }
            _ => {}
        }
    } else if head_button.rear.is_tapped() {
        mmng.stop_motion();
        nao_manager.set_all(
            nao_state.position.clone(),
            JointArray::<f32>::fill(-1.0),
            Priority::Critical,
        );
    }
    Ok(())
}
