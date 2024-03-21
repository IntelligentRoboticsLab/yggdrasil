use miette::Result;
use nidhogg::types::{FillExt, JointArray};
use nidhogg::NaoControlMessage;
use tyr::prelude::*;

use crate::filter::falling::{Fall, FallDirection, FallState};
use crate::filter::imu::IMUValues;
use crate::motion::motion_manager::MotionManager;
use crate::motion::motion_types::MotionType;

pub struct DamagePreventionModule;

impl Module for DamagePreventionModule {
    fn initialize(self, app: App) -> Result<App> {
        app.add_system(fallcatch)
            .add_resource(Resource::new(DamPrevResources {
                brace_for_impact: true,
            }))
    }
}

pub struct DamPrevResources {
    pub brace_for_impact: bool,
}

#[system]
fn fallcatch(
    fallingstate: &mut Fall,
    mmng: &mut MotionManager,
    damprevresources: &mut DamPrevResources,
    imu_values: &IMUValues,
    control: &mut NaoControlMessage,
) -> Result<()> {
    match fallingstate.state {
        FallState::Upright => damprevresources.brace_for_impact = true,
        FallState::Falling(_) => {
            if imu_values.angles.x > 0.8 || imu_values.angles.y > 0.8 {
                control.stiffness = JointArray::<f32>::fill(0.0);
            }
        }
        _ => (),
    }

    let selected_motion = match fallingstate.state {
        FallState::Falling(FallDirection::Forwards) => Some(MotionType::FallForwards),
        FallState::Falling(FallDirection::Backwards) => Some(MotionType::FallBackwards),
        FallState::Falling(FallDirection::Leftways) => Some(MotionType::FallLeftways),
        FallState::Falling(FallDirection::Rightways) => Some(MotionType::FallRightways),
        _ => None,
    };

    print!("{:?}\n\n", fallingstate.state);
    if damprevresources.brace_for_impact == true {
        if let Some(selected_motion) = selected_motion {
            mmng.start_new_motion(selected_motion);
            damprevresources.brace_for_impact = false;
        }
    }

    Ok(())
}
