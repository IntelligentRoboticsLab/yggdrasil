pub mod combinators;
pub mod conditions;
pub mod strategy;

use std::{future::Future, marker::PhantomData, sync::atomic::AtomicU32};

use bevy::{
    ecs::world::CommandQueue,
    prelude::*,
    tasks::{AsyncComputeTaskPool, ComputeTaskPool, IoTaskPool, Task, block_on},
    utils::futures::check_ready,
};
use strategy::{entity::EntityStrategy, resource::ResourceStrategy};

/// A tag that marks an entity as a running task.
#[derive(Component)]
pub struct YggdrasilTask(Task<CommandQueue>);

/// A tag that provides the type annotation for a running task.
#[derive(Component)]
pub struct Tag<T>(PhantomData<T>);

/// The generation of a task.
#[derive(Component, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Generation(u32);

/// The current generation of tasks.
///
/// The generation is incremented whenever a new set of tasks is spawned.
static CURRENT_GEN: AtomicU32 = AtomicU32::new(0);

/// Marker type for tasks that have no selected output method.
pub struct UnsetTask;
/// Marker type for tasks that output to resources.
pub struct ResourceTask;
/// Marker type for tasks that output to entities.
pub struct EntityTask;

/// Marker trait for task types.
pub trait TaskType {}

impl TaskType for ResourceTask {}
impl TaskType for EntityTask {}

/// Marker trait for pool types.
pub trait PoolType {
    fn pool() -> &'static TaskPool;
}

/// The pool on which a task is executed.
#[non_exhaustive]
pub enum TaskPool {
    Compute,
    AsyncCompute,
    Io,
}

impl TaskPool {
    #[must_use]
    pub fn get(&self) -> &'static bevy::tasks::TaskPool {
        match self {
            TaskPool::Compute => ComputeTaskPool::get(),
            TaskPool::AsyncCompute => AsyncComputeTaskPool::get(),
            TaskPool::Io => IoTaskPool::get(),
        }
    }
}

/// A builder for creating tasks. This is the entry point for new tasks.
pub struct TaskBuilder<'a, 'w, 's, Type> {
    commands: &'a mut Commands<'w, 's>,
    pool: TaskPool,
    _phantom: PhantomData<Type>,
}

impl<'a, 'w, 's> TaskBuilder<'a, 'w, 's, UnsetTask> {
    fn new(commands: &'a mut Commands<'w, 's>, pool: TaskPool) -> Self {
        Self {
            commands,
            pool,
            _phantom: PhantomData,
        }
    }
}

impl<'a, 'w, 's> TaskBuilder<'a, 'w, 's, UnsetTask> {
    #[must_use]
    pub fn to_resource(self) -> TaskBuilder<'a, 'w, 's, ResourceTask> {
        TaskBuilder::<'a, 'w, 's, _> {
            commands: self.commands,
            pool: self.pool,
            _phantom: PhantomData,
        }
    }

    #[must_use]
    pub fn to_entities(self) -> TaskBuilder<'a, 'w, 's, EntityTask> {
        TaskBuilder::<'a, 'w, 's, _> {
            commands: self.commands,
            pool: self.pool,
            _phantom: PhantomData,
        }
    }

    pub fn spawn_blocking<F, T>(&self, fut: F) -> T
    where
        F: Future<Output = T> + Send + 'static,
        T: Send + 'static,
    {
        let pool = self.pool.get();
        block_on(pool.spawn(fut))
    }
}

pub trait TaskFuture<T>: Future<Output = Option<T>> + Send + 'static {}
impl<F: Future<Output = Option<T>> + Send + 'static, T> TaskFuture<T> for F {}

impl TaskBuilder<'_, '_, '_, ResourceTask> {
    pub fn spawn_with_strategy<T: Resource, F: Future<Output = CommandQueue> + Send + 'static>(
        &mut self,
        strategy: impl ResourceStrategy<T, F> + 'static,
        task: impl TaskFuture<T>,
    ) {
        let task_pool = self.pool.get();

        let entity = self.commands.spawn_empty().id();

        let task = task_pool.spawn(async move { strategy(entity, task.await).await });

        self.commands
            .entity(entity)
            .insert((Tag(PhantomData::<T>), YggdrasilTask(task)));
    }

    pub fn spawn<T: Resource>(&mut self, task: impl TaskFuture<T>) {
        self.spawn_with_strategy(strategy::resource::to_resource, task);
    }
}

impl TaskBuilder<'_, '_, '_, EntityTask> {
    pub fn spawn_with_strategy<
        T: Send + Sync + 'static,
        F: Future<Output = CommandQueue> + Send + 'static,
    >(
        &mut self,
        strategy: impl EntityStrategy<T, F> + 'static,
        tasks: impl IntoIterator<Item = impl TaskFuture<T>> + Send + 'static,
    ) {
        let pool = AsyncComputeTaskPool::get();

        let generation = Generation(CURRENT_GEN.fetch_add(1, std::sync::atomic::Ordering::Relaxed));

        let tasks = tasks
            .into_iter()
            .map(|task| (self.commands.spawn_empty().id(), task))
            .map(|(entity, task)| {
                let strategy = strategy.clone();
                let generation = generation.clone();

                (entity, async move {
                    strategy(generation, entity, task.await).await
                })
            })
            // need to collect in between due to `problem case #3`
            .collect::<Vec<_>>();

        for (entity, future) in tasks {
            let task = pool.spawn(future);
            self.commands
                .entity(entity)
                .insert((Tag(PhantomData::<T>), YggdrasilTask(task)));
        }
    }

    pub fn spawn<T: Component>(
        &mut self,
        tasks: impl IntoIterator<Item = impl TaskFuture<T>> + Send + 'static,
    ) {
        self.spawn_with_strategy(strategy::entity::latest, tasks);
    }
}

pub trait CommandsExt<'a, 'w, 's> {
    fn prepare_task(&'a mut self, pool: TaskPool) -> TaskBuilder<'a, 'w, 's, UnsetTask>;
}

impl<'a, 'w, 's> CommandsExt<'a, 'w, 's> for Commands<'w, 's> {
    fn prepare_task(&'a mut self, pool: TaskPool) -> TaskBuilder<'a, 'w, 's, UnsetTask> {
        TaskBuilder::new(self, pool)
    }
}

fn handle_tasks(mut commands: Commands, mut query: Query<&mut YggdrasilTask>) {
    for mut task in &mut query {
        if let Some(mut command_queue) = check_ready(&mut task.0) {
            commands.append(&mut command_queue);
        }
    }
}

/// Plugin that provides the task system.
pub struct TaskPlugin;

impl Plugin for TaskPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostUpdate, handle_tasks);
    }
}
