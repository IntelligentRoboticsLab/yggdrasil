use std::time::Duration;

use miette::Result;
use tyr::{
    prelude::*,
    tasks::{AsyncDispatcher, Task, TaskModule},
};

#[derive(Default)]
struct Counter(u64);
struct Name(String);

// this is a function that needs to wait for a while, blocking our main thread
async fn receive_name(duration: Duration) -> Name {
    tokio::time::sleep(duration).await;
    Name("Daphne".to_string())
}

#[system]
fn dispatch_name(ad: &AsyncDispatcher, task: &mut Task<Name>) -> Result<()> {
    // If the task is alive already, we return early
    if task.is_alive() {
        return Ok(());
    }

    // We dispatch a future onto a separate thread, and set task
    // as alive by giving it the handle needed to poll it.
    ad.dispatch(&mut task, receive_name(Duration::from_secs(1)))?;

    Ok(())
}

#[system]
fn poll_name(task: &mut Task<Name>, counter: &mut Counter) -> Result<()> {
    // If the task hasn't completed yet, we return early
    let Some(name) = task.poll() else {
        return Ok(());
    };

    println!("Hello, {}! Counter is at {}", name.0, counter.0);
    counter.0 = 0;

    Ok(())
}

#[system]
fn time_critical_task(counter: &mut Counter) -> Result<()> {
    // This will still run many times a second even though
    // `receive_name` is waiting for 1 second
    counter.0 += 1;

    Ok(())
}

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    App::new()
        .add_module(TaskModule)?
        .add_resource(Resource::<Counter>::default())?
        .add_resource(Resource::<Task<Name>>::default())?
        // There's also a `.add_task_resource()` as a shorthand
        // for adding both a Resource<T> and Resource<Task<T>>.
        .add_system(dispatch_name)
        .add_system(poll_name)
        .add_system(time_critical_task)
        .run()?;

    Ok(())
}
