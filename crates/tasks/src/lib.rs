pub mod combinators;
pub mod conditions;
// TODO: open up strategy API (?)
mod strategy;

use std::{future::Future, marker::PhantomData, sync::atomic::AtomicU32};

use bevy::{
    ecs::world::CommandQueue,
    prelude::*,
    tasks::{AsyncComputeTaskPool, ComputeTaskPool, IoTaskPool, Scope, Task},
    utils::futures::check_ready,
};
use strategy::{entity::EntityStrategy, resource::ResourceStrategy};

#[derive(Component)]
pub struct TyrTask(Task<CommandQueue>);

#[derive(Component)]
pub struct Tag<T>(PhantomData<T>);

#[derive(Component, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Generation(u32);

static CURRENT_GEN: AtomicU32 = AtomicU32::new(0);

pub struct UnsetTask;
pub struct ResourceTask;
pub struct EntityTask;

pub trait TaskType {}

impl TaskType for ResourceTask {}
impl TaskType for EntityTask {}

pub trait PoolType {
    fn pool() -> &'static TaskPool;
}

pub struct UnsetPool;

pub enum TaskPool {
    Compute,
    AsyncCompute,
    Io,
}

impl TaskPool {
    pub fn get(&self) -> &'static bevy::tasks::TaskPool {
        match self {
            TaskPool::Compute => ComputeTaskPool::get(),
            TaskPool::AsyncCompute => AsyncComputeTaskPool::get(),
            TaskPool::Io => IoTaskPool::get(),
        }
    }
}

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
    pub fn to_resource(self) -> TaskBuilder<'a, 'w, 's, ResourceTask> {
        TaskBuilder::<'a, 'w, 's, _> {
            commands: self.commands,
            pool: self.pool,
            _phantom: PhantomData,
        }
    }

    pub fn to_entities(self) -> TaskBuilder<'a, 'w, 's, EntityTask> {
        TaskBuilder::<'a, 'w, 's, _> {
            commands: self.commands,
            pool: self.pool,
            _phantom: PhantomData,
        }
    }

    pub fn scope<'env, F, T>(&self, f: F) -> Vec<T>
    where
        F: for<'scope> FnOnce(&'scope Scope<'scope, 'env, T>),
        T: Send + 'static,
    {
        let pool = self.pool.get();
        pool.scope(f)
    }
}

pub trait TaskFuture<T>: Future<Output = Option<T>> + Send + 'static {}
impl<F: Future<Output = Option<T>> + Send + 'static, T> TaskFuture<T> for F {}

impl TaskBuilder<'_, '_, '_, ResourceTask> {
    fn spawn_with_strategy<T: Resource, F: Future<Output = CommandQueue> + Send + 'static>(
        &mut self,
        strategy: impl ResourceStrategy<T, F> + 'static,
        task: impl TaskFuture<T>,
    ) {
        let task_pool = self.pool.get();

        let entity = self.commands.spawn_empty().id();

        let task = task_pool.spawn(async move { strategy(entity, task.await).await });

        self.commands
            .entity(entity)
            .insert((Tag(PhantomData::<T>), TyrTask(task)));
    }

    pub fn spawn<T: Resource>(&mut self, task: impl TaskFuture<T>) {
        self.spawn_with_strategy(strategy::resource::to_resource, task);
    }
}

impl TaskBuilder<'_, '_, '_, EntityTask> {
    fn spawn_with_strategy<
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

        tasks.into_iter().for_each(|(entity, future)| {
            let task = pool.spawn(future);
            self.commands
                .entity(entity)
                .insert((Tag(PhantomData::<T>), TyrTask(task)));
        });
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

fn handle_tasks(mut commands: Commands, mut query: Query<&mut TyrTask>) {
    for mut task in query.iter_mut() {
        if let Some(mut command_queue) = check_ready(&mut task.0) {
            commands.append(&mut command_queue);
        }
    }
}

pub struct TaskPlugin;

impl Plugin for TaskPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostUpdate, handle_tasks);
    }
}
