use std::time::Duration;

use miette::Result;
use tyr::prelude::*;
use tyr::tasks::{Error, TaskConfig, TaskModule};

#[derive(Default)]
struct Counter(u64);
struct Name(String);

// this is a function that needs to wait for a while, blocking our main thread
async fn receive_name(duration: Duration) -> Name {
    tokio::time::sleep(duration).await;
    Name("Daphne".to_string())
}

#[system]
fn dispatch_name(task: &mut AsyncTask<Name>) -> Result<()> {
    // Dispatches a future to a background thread where it can be efficiently
    // awaited without blocking all the other systems and tasks.
    //
    // Also marks the task as active, so we can't accidentally dispatch it twice.
    match task.try_spawn(receive_name(Duration::from_secs(1))) {
        // Dispatched!
        Ok(_) | Err(Error::NotActive) => Ok(()),
        // This is also fine here, we were already running the task from another cycle
        // and can return without dispatching it again
        Err(Error::AlreadyActive) => Ok(()),
    }
}

#[system]
fn poll_name(task: &mut AsyncTask<Name>, counter: &mut Counter) -> Result<()> {
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
    let task_config = TaskConfig {
        async_threads: 1,
        compute_threads: 1,
    };

    App::new()
        .add_resource(Resource::new(task_config))?
        .add_module(TaskModule)?
        .init_resource::<Counter>()?
        .add_task::<AsyncTask<Name>>()?
        .add_system(dispatch_name)
        .add_system(poll_name)
        .add_system(time_critical_task)
        .run()
}
