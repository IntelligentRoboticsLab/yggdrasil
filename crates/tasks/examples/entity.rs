#![allow(clippy::disallowed_names)]
use std::time::Duration;

use async_std::task;
use bevy::{diagnostic::FrameCount, prelude::*};
use rand::Rng;
use tasks::{CommandsExt, TaskPlugin, TaskPool, conditions::task_finished};

#[derive(Component, Debug)]
struct Foo {
    frame: FrameCount,
    number: u32,
}

fn run_task_entity(mut commands: Commands, frame: Res<FrameCount>) {
    commands
        .prepare_task(TaskPool::AsyncCompute)
        .to_entities()
        .spawn({
            let frame = *frame;

            (0..5).map(move |number| async move {
                let duration = rand::rng().random_range(0..5);

                task::sleep(Duration::from_secs(duration)).await;

                Some(Foo { frame, number })
            })
        });
}

fn query_foo_entity(foo: Query<&Foo, Added<Foo>>) {
    for Foo { frame, number } in foo.iter() {
        println!("Foo #{}! Dispatched in frame: {}", number, frame.0);
    }
}

fn main() {
    App::new()
        .add_plugins((MinimalPlugins, TaskPlugin))
        .add_systems(
            Update,
            (
                run_task_entity.run_if(task_finished::<Foo>),
                query_foo_entity,
            ),
        )
        .run();
}
