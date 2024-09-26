use bevy::prelude::*;
use tasks::*;

use super::{
    backend::{InferRequest, ModelExecutor},
    data_type::Output,
    MlModel,
};

pub trait MlInferenceBuilderState {}

pub struct MlInferenceBuilder<'a, 'w, 's, M: MlModel, S: MlInferenceBuilderState> {
    commands: &'a mut Commands<'w, 's>,
    executor: &'a mut ModelExecutor<M>,
    state: S,
}

pub struct DefineInput;
impl MlInferenceBuilderState for DefineInput {}

pub struct DefineOutput<'a, M: MlModel>(&'a [M::InputType]);
impl<'a, M: MlModel> MlInferenceBuilderState for DefineOutput<'a, M> {}

pub struct DefineBatchedOutput<'a, M: MlModel>(&'a [&'a [M::InputType]]);
impl<'a, M: MlModel> MlInferenceBuilderState for DefineBatchedOutput<'a, M> {}

pub struct ResourceOutput<'a, M: MlModel>(&'a [M::InputType]);
impl<'a, M: MlModel> MlInferenceBuilderState for ResourceOutput<'a, M> {}

pub struct EntityOutput<'a, M: MlModel>(&'a [M::InputType]);
impl<'a, M: MlModel> MlInferenceBuilderState for EntityOutput<'a, M> {}

pub struct EntitiesOutput<'a, M: MlModel>(&'a [&'a [M::InputType]]);
impl<'a, M: MlModel> MlInferenceBuilderState for EntitiesOutput<'a, M> {}

pub trait MlTaskCommandsExt<'a, 'w, 's> {
    fn infer_model<M: MlModel>(
        &'a mut self,
        executor: &'a mut ModelExecutor<M>,
    ) -> MlInferenceBuilder<'a, 'w, 's, M, DefineInput>;
}

impl<'a, 'w, 's> MlTaskCommandsExt<'a, 'w, 's> for Commands<'w, 's> {
    fn infer_model<M: MlModel>(
        &'a mut self,
        executor: &'a mut ModelExecutor<M>,
    ) -> MlInferenceBuilder<'a, 'w, 's, M, DefineInput> {
        MlInferenceBuilder {
            commands: self,
            executor,
            state: DefineInput,
        }
    }
}

impl<'a, 'w, 's, M: MlModel> MlInferenceBuilder<'a, 'w, 's, M, DefineInput> {
    pub fn with_input(
        &'a mut self,
        input: &'a [M::InputType],
    ) -> MlInferenceBuilder<'a, 'w, 's, M, DefineOutput<'a, M>> {
        MlInferenceBuilder {
            commands: self.commands,
            executor: self.executor,
            state: DefineOutput(input),
        }
    }

    pub fn with_batched_input(
        &'a mut self,
        input: &'a [&'a [M::InputType]],
    ) -> MlInferenceBuilder<'a, 'w, 's, M, DefineBatchedOutput<'a, M>> {
        MlInferenceBuilder {
            commands: self.commands,
            executor: self.executor,
            state: DefineBatchedOutput(input),
        }
    }
}

impl<'a, 'w, 's, M: MlModel> MlInferenceBuilder<'a, 'w, 's, M, DefineOutput<'a, M>> {
    pub fn to_resource(self) -> MlInferenceBuilder<'a, 'w, 's, M, ResourceOutput<'a, M>> {
        MlInferenceBuilder {
            commands: self.commands,
            executor: self.executor,
            state: ResourceOutput(self.state.0),
        }
    }

    pub fn to_entity(self) -> MlInferenceBuilder<'a, 'w, 's, M, EntityOutput<'a, M>> {
        MlInferenceBuilder {
            commands: self.commands,
            executor: self.executor,
            state: EntityOutput(self.state.0),
        }
    }
}

impl<'a, 'w, 's, M: MlModel> MlInferenceBuilder<'a, 'w, 's, M, ResourceOutput<'a, M>> {
    pub fn spawn<O, F, R>(&mut self, f: F)
    where
        O: Output<M::OutputType>,
        F: (FnOnce(Vec<O>) -> R) + Send + Sync + 'static,
        R: Resource,
    {
        let request = self
            .executor
            .request_infer(&[&self.state.0])
            .expect("failed to request inference");

        self.commands
            .prepare_task(TaskPool::AsyncCompute)
            .to_resource()
            .spawn({
                async move {
                    let output = request.run().ok()?;
                    let output = output.fetch_output().ok()?;

                    Some(f(output))
                }
            });
    }
}

impl<'a, 'w, 's, M: MlModel> MlInferenceBuilder<'a, 'w, 's, M, EntityOutput<'a, M>> {
    pub fn spawn<O, F, C>(&mut self, f: F)
    where
        O: Output<M::OutputType>,
        F: (FnOnce(Vec<O>) -> C) + Send + Sync + 'static,
        C: Component,
    {
        let request = self
            .executor
            .request_infer(&[&self.state.0])
            .expect("failed to request inference");

        self.commands
            .prepare_task(TaskPool::AsyncCompute)
            .to_entities()
            .spawn({
                vec![async move {
                    let output = request.run().ok()?;
                    let output = output.fetch_output().ok()?;

                    Some(f(output))
                }]
            });
    }
}

impl<'a, 'w, 's, M: MlModel> MlInferenceBuilder<'a, 'w, 's, M, DefineBatchedOutput<'a, M>> {
    pub fn to_entities(self) -> MlInferenceBuilder<'a, 'w, 's, M, EntitiesOutput<'a, M>> {
        MlInferenceBuilder {
            commands: self.commands,
            executor: self.executor,
            state: EntitiesOutput(self.state.0),
        }
    }
}

impl<'a, 'w, 's, M: MlModel + Send + Sync + 'static>
    MlInferenceBuilder<'a, 'w, 's, M, EntitiesOutput<'a, M>>
{
    pub fn spawn<O, F, C>(&mut self, f: F)
    where
        O: Output<M::OutputType>,
        F: (FnOnce(Vec<O>) -> C) + Clone + Copy + Send + Sync + 'static,
        C: Component,
    {
        let requests = self
            .state
            .0
            .iter()
            .map(|input| {
                self.executor
                    .request_infer(&[input])
                    .expect("failed to create inference request")
            })
            .collect::<Vec<InferRequest<M>>>();

        self.commands
            .prepare_task(TaskPool::AsyncCompute)
            .to_entities()
            .spawn({
                requests.into_iter().map(move |request| async move {
                    async move {
                        let output = request.run().ok()?;
                        let output = output.fetch_output().ok()?;

                        Some(f(output))
                    }
                    .await
                })
            })
    }
}
