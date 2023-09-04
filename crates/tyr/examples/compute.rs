use std::time::Duration;

use miette::Result;
use tyr::{
    prelude::*,
    tasks::{ComputeDispatcher, Task, TaskModule},
};

#[derive(Default)]
struct Counter(u64);
struct Name(String);

// this is an expensive function that needs a lot of calculation, blocking our main thread
fn calculate_name(duration: Duration) -> Name {
    std::thread::sleep(duration);
    Name("Daphne".to_string())
}

#[system]
fn dispatch_name(cd: &ComputeDispatcher, task: &mut Task<Name>) -> Result<()> {
    // We dispatch a function onto a threadpool, and set task
    // as alive by giving it the handle needed to poll it.
    let _ = cd.try_dispatch(&mut task, move || calculate_name(Duration::from_secs(1)));
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
    // `calculate_name` is sleeping for 1 second
    counter.0 += 1;

    Ok(())
}

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    miette::set_panic_hook();

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
