use crate::{
    behavior::engine::{Behavior, Context, Control},
    nao::manager::Priority,
};
use nidhogg::types::{FillExt, HeadJoints};

const HEAD_STIFFNESS: f32 = 0.3;

#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct Stand;

impl Behavior for Stand {
    fn execute(&mut self, _context: Context, control: &mut Control) {
        control.walking_engine.request_stand();
        control.walking_engine.end_step_phase();

        control.nao_manager.set_head(
            HeadJoints::default(),
            HeadJoints::fill(HEAD_STIFFNESS),
            Priority::default(),
        );
    }
}
