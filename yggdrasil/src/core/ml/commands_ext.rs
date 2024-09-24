use bevy::prelude::*;
use tasks::*;

use super::{backend::ModelExecutor, MlModel};

pub trait MlTaskCommandsExt<'a, 'w> {
    fn infer_model<M: MlModel>(&mut self, executor: ModelExecutor<M>) -> &mut Self;
}

impl<'a, 'w> MlTaskCommandsExt<'a, 'w> for Commands<'a, 'w> {
    fn infer_model<M: MlModel>(&mut self, executor: ModelExecutor<M>) -> &mut Self {
        self.prepare_task();
        self
    }
}
