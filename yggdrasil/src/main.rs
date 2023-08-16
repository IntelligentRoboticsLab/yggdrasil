pub mod r#async;
pub mod event;
pub mod filter;
pub mod nao;

use std::time::{Duration, Instant};

use event::Event;
// use filter::FilterModule;
use miette::Result;
// use nao::NaoModule;
use r#async::{AsyncDispatcher, AsyncModule, AsyncResource, AsyncTask};
use tyr::prelude::*;

#[derive(Debug)]
struct Cheese;

#[derive(Debug)]
struct Benchmark {
    poll_count: u64,
    start_time: Instant,
}

#[system]
fn send_cheese(
    dispatcher: &mut AsyncDispatcher,
    task: &mut AsyncTask<Cheese>,
    bench: &mut Benchmark,
) -> Result<()> {
    // Task is already active
    if task.is_alive() {
        return Ok(());
    }

    // Spawn a new task (async fn)
    async fn get_cheese() -> Cheese {
        tokio::time::sleep(Duration::from_millis(100)).await;
        Cheese
    }

    task.spawn(dispatcher.dispatch(get_cheese()));

    bench.poll_count = 0;
    bench.start_time = Instant::now();

    Ok(())
}

#[system]
fn store_async_on_completion<T: Send + Sync + 'static>(
    task: &mut AsyncTask<T>,
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
        .add_async_resource(Resource::new(Cheese))?
        .add_resource(Resource::new(Benchmark {
            poll_count: 0,
            start_time: Instant::now(),
        }))?
        // .add_module(NaoModule)?
        // .add_module(FilterModule)?
        .add_system(send_cheese)
        .add_system(store_async_on_completion::<Cheese>.after(send_cheese))
        .run()?;
    Ok(())
}
