use std::future::Future;

use crate::{Tag, YggdrasilTask};
use bevy::{ecs::world::CommandQueue, prelude::*};

pub trait ResourceStrategy<T, F: Future<Output = CommandQueue> + Send + 'static>:
    Fn(Entity, Option<T>) -> F + Send + Sync + 'static
{
}
impl<
    T,
    F: Future<Output = CommandQueue> + Send + 'static,
    S: Fn(Entity, Option<T>) -> F + Send + Sync + 'static,
> ResourceStrategy<T, F> for S
{
}

pub async fn to_resource<T: Resource>(entity: Entity, value: Option<T>) -> CommandQueue {
    let mut queue = CommandQueue::default();

    queue.push(move |world: &mut World| {
        if let Some(value) = value {
            world.insert_resource(value);
        }

        world.entity_mut(entity).remove::<(Tag<T>, YggdrasilTask)>();
    });

    queue
}
