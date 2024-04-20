use crate::{
    behavior::engine::{Behavior, Context},
    nao::manager::{NaoManager, Priority},
    walk::engine::WalkingEngine,
    motion::{motion_manager::MotionManager, step_planner::StepPlanner},
};

use nidhogg::types::{color, FillExt, RightEye};

#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct EnergyEfficientStand{
    pub standing: bool,
}

impl Behavior for EnergyEfficientStand {
    fn execute(
        &mut self,
        context: Context,
        nao_manager: &mut NaoManager,
        walking_engine: &mut WalkingEngine,
        _motion_manager: &mut MotionManager,
        _step_planner: &mut StepPlanner,
    ) {
        nao_manager.set_right_eye_led(RightEye::fill(color::f32::GREEN), Priority::default());

        if !walking_engine.is_standing() && !self.standing {
            walking_engine.request_stand();
            walking_engine.end_step_phase();
            println!("Not standing");

        } 
        else {
            self.standing = true;
            let request = &context.noa_control_message.position;
            let currents = &context.nao_state.current;
            let position = &context.nao_state.position;
            let head_stiffness = context.noa_control_message.stiffness.head_joints();
            let arm_stiffness = context.noa_control_message.stiffness.arm_joints();
            let leg_stiffness = context.noa_control_message.stiffness.leg_joints();
            let tempuratur = &context.nao_state.temperature;

            println!("{:?}", currents);
            // println!("{:?}", request);
            // println!("{:?}", position);
            println!("{:?}", tempuratur);
            let threshold: f32 = 0.1;
            let offset: f32 = 0.0001;
            let new_request = currents.clone()
                .zip(request.clone())
                .zip(position.clone())
                .map(move |((x, y), z)| if x > threshold {if y > z {y-offset} else {y + offset}} else {y});
            nao_manager.set_all(new_request, head_stiffness, arm_stiffness, leg_stiffness, Priority::Critical);
        }
        // else {
        // //     self.standing = true;
        // //     let request = &context.noa_control_message.position;
        // //     let currents = &context.nao_state.current;
        // //     let position = &context.nao_state.position;
        // //     let head_stiffness = context.noa_control_message.stiffness.head_joints();
        // //     let arm_stiffness = context.noa_control_message.stiffness.arm_joints();
        // //     let leg_stiffness = context.noa_control_message.stiffness.leg_joints();
        //     let tempuratur = &context.nao_state.temperature;

        // //     // println!("{:?}", currents);
        // //     // println!("{:?}", request);
        // //     // println!("{:?}", position);
        //     println!("{:?}", tempuratur);
        // //     let threshold: f32 = 0.3;
        // //     let offset: f32 = 0.00001;
        // //     let new_request = currents.clone()
        // //         .zip(request.clone())
        // //         .zip(position.clone())
        // //         .map(move |((x, y), z)| if x > threshold {if y > z {return y-offset} else {return y + offset}} else {return y});
        // //     nao_manager.set_all(new_request, head_stiffness, arm_stiffness, leg_stiffness, Priority::Critical);
        // }
    }
}


