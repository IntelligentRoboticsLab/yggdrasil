use std::time::Duration;

use bevy::{core::FrameCount, prelude::*};
use tasks::{conditions::task_finished, CommandsExt, TaskPlugin, TaskPool};

#[derive(Resource, Default)]
pub struct Foo(FrameCount);

fn run_task_resource(mut commands: Commands, frame: Res<FrameCount>) {
    commands
        // Select a pool to spawn the task in
        .prepare_task(TaskPool::AsyncCompute)
        // Select the output type of the task. There are three options:
        //
        // - to_resource() will insert the resulting Some(T) into Res<T>
        // - to_entities() will insert the resulting Some(T) into a new entity with component T
        // - scope() will block on the inputted task and return a Vec<T> directly
        .to_resource()
        // Spawn the given task on the pool
        .spawn({
            let frame = frame.clone();
            async move {
                async_std::task::sleep(Duration::from_secs(3)).await;
                Some(Foo(frame))
            }
        });
}

fn query_foo_resource(foo: Res<Foo>) {
    if !foo.is_changed() || foo.is_added() {
        return;
    }

    println!("Foo has changed! Dispatched in frame: {:?}", foo.0 .0);
}

fn main() {
    App::new()
        .add_plugins((MinimalPlugins, TaskPlugin))
        .init_resource::<Foo>()
        .add_systems(
            Update,
            (
                run_task_resource.run_if(task_finished::<Foo>),
                query_foo_resource,
            ),
        )
        .run();
}
