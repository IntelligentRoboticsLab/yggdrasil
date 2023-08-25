use std::time::{Duration, Instant};

use miette::Result;
use tyr::{
    prelude::*,
    tasks::{ComputeDispatcher, Task, TaskModule, TaskResource},
};

#[derive(Debug)]
struct Cheese;

#[derive(Debug)]
struct Benchmark {
    poll_count: u64,
    start_time: Instant,
}

// Spawn a new compute task
fn calculate_cheese(duration: Duration) -> Cheese {
    std::thread::sleep(duration);
    Cheese
}

#[system]
fn send_cheese(
    cd: &ComputeDispatcher,
    task: &mut Task<Cheese>,
    bench: &mut Benchmark,
) -> Result<()> {
    // Task is already active
    if task.is_alive() {
        return Ok(());
    }

    let duration = Duration::from_millis(100);
    cd.dispatch(&mut task, move || calculate_cheese(duration))?;

    // Reset benchmark counters
    bench.poll_count = 0;
    bench.start_time = Instant::now();

    Ok(())
}

#[system]
fn store_task_on_completion<T: Send + Sync + 'static>(
    task: &mut Task<T>,
    resource: &mut T,
    bench: &mut Benchmark,
) -> Result<()> {
    if let Some(result) = task.poll() {
        *resource = result;
        println!(
            "Poll count: {}, time taken: {}ms",
            bench.poll_count,
            bench.start_time.elapsed().as_millis()
        );
    } else {
        bench.poll_count += 1;
    }

    Ok(())
}

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    App::new()
        .add_module(TaskModule)?
        .add_task_resource(Resource::new(Cheese))?
        .add_resource(Resource::new(Benchmark {
            poll_count: 0,
            start_time: Instant::now(),
        }))?
        .add_system(send_cheese)
        .add_system(store_task_on_completion::<Cheese>.after(send_cheese))
        .run()?;
    Ok(())
}
