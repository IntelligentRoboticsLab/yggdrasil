use std::time::Duration;

use bevy::{prelude::*, time::Stopwatch};
use tasks::{CommandsExt, TaskPlugin, TaskPool};

fn run_blocking_task(
    mut commands: Commands,
    mut counter: Local<usize>,
    time: Res<Time>,
    mut stopwatch: Local<Stopwatch>,
) {
    stopwatch.tick(time.delta());

    if stopwatch.elapsed() < Duration::from_secs(3) {
        return;
    }

    stopwatch.reset();

    // Blocks on the task and return the result directly
    let output = commands
        .prepare_task(TaskPool::AsyncCompute)
        .spawn_blocking({
            let counter = *counter;
            async move { counter + 1 }
        });

    *counter = output;

    println!("Counter: {}", *counter);
}

fn main() {
    App::new()
        .add_plugins((MinimalPlugins, TaskPlugin))
        .add_systems(Update, run_blocking_task)
        .run();
}
