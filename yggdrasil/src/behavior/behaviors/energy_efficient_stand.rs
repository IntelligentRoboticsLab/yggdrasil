use crate::{
    behavior::engine::{Behavior, Context},
    nao::manager::{NaoManager, Priority},
    walk::engine::WalkingEngine,
};

use nidhogg::types::{color, FillExt, RightEye};

#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct EnergyEfficientStand;

impl Behavior for EnergyEfficientStand {
    fn execute(
        &mut self,
        context: Context,
        nao_manager: &mut NaoManager,
        walking_engine: &mut WalkingEngine
    ) {
        nao_manager.set_right_eye_led(RightEye::fill(color::f32::GREEN), Priority::default());

        if !walking_engine.is_standing() {
            walking_engine.request_stand();
            walking_engine.end_step_phase();
        } else {
            let request = &context.noa_control_message.position;
            let currents = &context.nao_state.current;
            let position = &context.nao_state.position;
            println!("{:?}", currents);
            println!("{:?}", request);
            println!("{:?}", position);
            let threshold: &f32 = &0.1;
            for i in 0..25 {
                if currents.into_iter() >= threshold {
                    
                }
            }
        }

        nao_manager
            .unstiff_arms(Priority::Critical)
            .unstiff_head(Priority::Critical);
    }
}


