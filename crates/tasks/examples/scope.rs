use std::time::Duration;

use bevy::{prelude::*, time::Stopwatch};
use tasks::{CommandsExt, TaskPlugin, TaskPool};

fn run_scoped_task(
    mut commands: Commands,
    mut counter: Local<usize>,
    time: Res<Time>,
    mut stopwatch: Local<Stopwatch>,
) {
    stopwatch.tick(time.delta());

    if stopwatch.elapsed() < Duration::from_secs(3) {
        return;
    };

    stopwatch.reset();

    // Blocks on the task and return the result directly
    let output = commands.prepare_task(TaskPool::AsyncCompute).scope({
        let counter = counter.clone();
        move |s| s.spawn(async move { counter + 1 })
    });

    *counter = output[0];

    println!("Counter: {}", *counter);
}

fn main() {
    App::new()
        .add_plugins((MinimalPlugins, TaskPlugin))
        .add_systems(Update, run_scoped_task)
        .run();
}
