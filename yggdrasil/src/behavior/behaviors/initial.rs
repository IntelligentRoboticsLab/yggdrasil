use crate::{
    behavior::engine::{Behavior, Context},
    nao::manager::NaoManager,
};

#[derive(Copy, Clone, Debug, Default)]
pub struct Initial;

impl Behavior for Initial {
    fn execute(&mut self, _context: Context, _nao_manager: &mut NaoManager) {}
}
