use std::future::Future;

use crate::{Generation, Tag, YggdrasilTask};
use bevy::{ecs::world::CommandQueue, prelude::*, utils::BoxedFuture};

pub trait EntityStrategy<T, F: Future<Output = CommandQueue> + Send + 'static>:
    Fn(Generation, Entity, Option<T>) -> F + Clone + Send + Sync + 'static
{
}
impl<
    T,
    F: Future<Output = CommandQueue> + Send + 'static,
    S: Fn(Generation, Entity, Option<T>) -> F + Clone + Send + Sync + 'static,
> EntityStrategy<T, F> for S
{
}

pub use self::keep_all as just_add_the_shit;

pub async fn keep_all<T: Component>(
    generation: Generation,
    entity: Entity,
    value: Option<T>,
) -> CommandQueue {
    let mut queue = CommandQueue::default();

    queue.push(move |world: &mut World| {
        if let Some(value) = value {
            world
                .entity_mut(entity)
                .insert((value, generation))
                .remove::<(Tag<T>, YggdrasilTask)>();
        } else {
            world.entity_mut(entity).despawn();
        }
    });

    queue
}

pub async fn latest<T: Component>(
    generation: Generation,
    entity: Entity,
    value: Option<T>,
) -> CommandQueue {
    latest_n(1)(generation, entity, value).await
}

pub fn latest_n<T: Send + Component>(
    n: usize,
) -> impl Fn(Generation, Entity, Option<T>) -> BoxedFuture<'static, CommandQueue> + Clone {
    #[allow(clippy::unused_async)]
    pub async fn to_entity_latest_n_inner<T: Send + Component>(
        n: usize,
        generation: Generation,
        entity: Entity,
        value: Option<T>,
    ) -> CommandQueue {
        let mut queue = CommandQueue::default();

        queue.push(move |world: &mut World| {
            let mut old_entities = world.query::<(Entity, &Generation)>();

            let old_generations: Vec<&Generation> = {
                let mut generations: Vec<_> = old_entities
                    .iter(world)
                    .map(|(_, generation)| generation)
                    .collect();

                generations.sort();
                generations.dedup();
                let new_length = generations.len().saturating_sub(n);
                generations.truncate(new_length);
                generations
            };

            let to_despawn = old_entities
                .iter(world)
                .filter(|(_, generation)| old_generations.contains(generation))
                .map(|(entity, _)| entity)
                .collect::<Vec<_>>();

            for entity in to_despawn {
                world.entity_mut(entity).despawn();
            }

            if let Some(value) = value {
                world
                    .entity_mut(entity)
                    .insert((value, generation))
                    .remove::<(Tag<T>, YggdrasilTask)>();
            } else {
                world.entity_mut(entity).despawn();
            }
        });

        queue
    }

    move |generation, entity, value| {
        Box::pin(to_entity_latest_n_inner(n, generation, entity, value))
    }
}
