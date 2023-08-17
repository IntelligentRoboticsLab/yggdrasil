use std::time::{Duration, Instant};

use miette::Result;
use tyr::{
    event::Event,
    prelude::*,
    r#async::{AsyncDispatcher, AsyncModule},
    task::{Task, TaskResource},
};

#[derive(Debug)]
struct Cheese;

#[derive(Debug)]
struct Benchmark {
    poll_count: u64,
    start_time: Instant,
}

// Spawn a new compute task
async fn calculate_cheese(sleep_duration: Duration) -> Cheese {
    tokio::time::sleep(sleep_duration).await;
    Cheese
}

#[system]
fn send_cheese(cd: &AsyncDispatcher, task: &mut Task<Cheese>, bench: &mut Benchmark) -> Result<()> {
    // Task is already active
    if task.is_alive() {
        return Ok(());
    }

    task.spawn(cd.dispatch(calculate_cheese(Duration::from_millis(100))));

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
        .add_module(AsyncModule)?
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
