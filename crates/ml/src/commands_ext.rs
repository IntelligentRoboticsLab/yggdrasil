//! Defines extension methods for [`Commands`] to spawn inference tasks for machine learning models.

use bevy::prelude::*;
use tasks::*;

use super::{
    backend::{InferRequest, ModelExecutor},
    element_type::Output,
    MlModel,
};

/// Type state for the inference builder.
pub trait MlInferenceBuilderState {}

/// Builder for spawning an [`MlModel`] inference task.
pub struct MlInferenceBuilder<'a, 'w, 's, M: MlModel, S: MlInferenceBuilderState> {
    commands: &'a mut Commands<'w, 's>,
    executor: &'a mut ModelExecutor<M>,
    state: S,
}

/// Type state for defining the input of the model.
pub struct DefineInput;
impl MlInferenceBuilderState for DefineInput {}

/// Type state for defining the output of the model.
pub struct DefineOutput<'a, M: MlModel>(&'a [M::InputType]);
impl<'a, M: MlModel> MlInferenceBuilderState for DefineOutput<'a, M> {}

/// Type state for defining the output of the model with multiple batches.
pub struct DefineBatchedOutput<'a, M: MlModel>(&'a [&'a [M::InputType]]);
impl<'a, M: MlModel> MlInferenceBuilderState for DefineBatchedOutput<'a, M> {}

/// Type state for storing the output of the model in a resource.
pub struct ResourceOutput<'a, M: MlModel>(&'a [M::InputType]);
impl<'a, M: MlModel> MlInferenceBuilderState for ResourceOutput<'a, M> {}

/// Type state for storing the output of the model in a single entity.
pub struct EntityOutput<'a, M: MlModel>(&'a [M::InputType]);
impl<'a, M: MlModel> MlInferenceBuilderState for EntityOutput<'a, M> {}

/// Type state for storing the output of the model in multiple entities.
pub struct EntitiesOutput<'a, M: MlModel>(&'a [&'a [M::InputType]]);
impl<'a, M: MlModel> MlInferenceBuilderState for EntitiesOutput<'a, M> {}

/// Extension trait for [`Commands`] to spawn an [`MlModel`] inference tasks.
pub trait MlTaskCommandsExt<'a, 'w, 's> {
    /// Begin building an inference task for an [`MlModel`].
    ///
    /// This allows you to define the input, output, and how the output should be stored.
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
    /// Define the input of the model with a single batch.
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

    /// Define the input of the model with multiple batches.
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
    /// Send the output of the model to a [`Resource`] will be spawned as soon as the model
    /// inference is complete.
    pub fn to_resource(self) -> MlInferenceBuilder<'a, 'w, 's, M, ResourceOutput<'a, M>> {
        MlInferenceBuilder {
            commands: self.commands,
            executor: self.executor,
            state: ResourceOutput(self.state.0),
        }
    }

    /// A new entity that will be spawned with the output attached
    /// as components as soon as the model inference is complete.
    pub fn to_entity(self) -> MlInferenceBuilder<'a, 'w, 's, M, EntityOutput<'a, M>> {
        MlInferenceBuilder {
            commands: self.commands,
            executor: self.executor,
            state: EntityOutput(self.state.0),
        }
    }
}

impl<'a, 'w, 's, M: MlModel> MlInferenceBuilder<'a, 'w, 's, M, ResourceOutput<'a, M>> {
    /// Spawn the model inference task, providing a closure to convert the output to a [`Resource`].
    pub fn spawn<O, F, R>(&mut self, f: F)
    where
        O: Output<M::OutputType>,
        F: (FnOnce(O) -> Option<R>) + Send + Sync + 'static,
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
                    // TODO: Add back support for multiple outputs
                    let output = request
                        .run()
                        .and_then(|request| request.fetch_output())
                        .map(|mut out| out.remove(0))
                        .ok()?;

                    f(output)
                }
            });
    }
}

impl<'a, 'w, 's, M: MlModel> MlInferenceBuilder<'a, 'w, 's, M, EntityOutput<'a, M>> {
    /// Spawn the model inference task, providing a closure to convert the output to a [`Component`].
    pub fn spawn<O, F, C>(&mut self, f: F)
    where
        O: Output<M::OutputType>,
        F: (FnOnce(Vec<O>) -> Option<C>) + Send + Sync + 'static,
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

                    f(output)
                }]
            });
    }
}

impl<'a, 'w, 's, M: MlModel> MlInferenceBuilder<'a, 'w, 's, M, DefineBatchedOutput<'a, M>> {
    /// Spawn an entity with the output attached a component for each batch.
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
    /// Spawn the model inference task, providing a closure to convert the output to a [`Component`].
    pub fn spawn<O, F, C>(&mut self, f: F)
    where
        O: Output<M::OutputType>,
        F: (FnOnce(Vec<O>) -> Option<C>) + Clone + Copy + Send + Sync + 'static,
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

                        f(output)
                    }
                    .await
                })
            })
    }
}
