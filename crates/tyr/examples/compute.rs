use std::time::Duration;

use miette::Result;
use tyr::prelude::*;
use tyr::tasks::{Error, TaskConfig, TaskModule};

#[derive(Default)]
struct Counter(u64);
struct Name(String);

// this is an expensive function that needs a lot of calculation, blocking our main thread
fn calculate_name(duration: Duration) -> Name {
    std::thread::sleep(duration);
    Name("Daphne".to_string())
}

#[system]
fn dispatch_name(task: &mut ComputeTask<Name>) -> Result<()> {
    // We dispatch a function onto a threadpool where it runs without blocking
    // other systems.
    //
    // Also marks the task as active, so we can't accidentally dispatch it twice.
    match task.try_spawn(move || calculate_name(Duration::from_secs(1))) {
        // Dispatched!
        Ok(_) | Err(Error::NotActive) => Ok(()),
        // This is also fine here, we were already running the task from another cycle
        // and can return without dispatching it again
        Err(Error::AlreadyActive) => Ok(()),
    }
}

#[system]
fn poll_name(task: &mut ComputeTask<Name>, counter: &mut Counter) -> Result<()> {
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
    // `calculate_name` is sleeping for 1 second
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
        .add_task::<ComputeTask<Name>>()?
        .add_system(dispatch_name)
        .add_system(poll_name)
        .add_system(time_critical_task)
        .run()?;

    Ok(())
}
