pub mod r#async;
pub mod filter;
pub mod nao;

use std::time::Duration;

// use filter::FilterModule;
use miette::Result;
// use nao::NaoModule;
use r#async::{
    poll_task,
    runtime::{AsyncDispatcher, Task},
    AsyncModule,
};
use tyr::prelude::*;

#[derive(Debug)]
struct Cheese;

#[system]
fn send_cheese(dispatcher: &mut AsyncDispatcher, task: &mut Option<Task<Cheese>>) -> Result<()> {
    // Task is already active
    if task.is_some() {
        return Ok(());
    }

    // Spawn a new task (async block)
    // let new_task = dispatcher.spawn(async {
    //     tokio::time::sleep(Duration::from_secs(1)).await;
    //     Cheese
    // });

    // Spawn a new task (async fn)
    async fn get_cheese() -> Cheese {
        tokio::time::sleep(Duration::from_secs(1)).await;
        Cheese
    }

    let new_task = dispatcher.spawn(get_cheese());

    *task = Some(new_task);

    Ok(())
}

#[system]
fn receive_cheese(task: &mut Option<Task<Cheese>>) -> Result<()> {
    let inner = task.as_mut().unwrap();

    // TODO: make systems conditional based on task status
    // - Runs with injected task result on completion
    // - Does not run in pending state
    if let Some(cheese) = poll_task(inner) {
        println!("You got some {cheese:?}!");
        *task = None;
    }

    Ok(())
}

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    App::new()
        // TODO: Some kind of task/general event storage apart from resources?
        .add_resource(Resource::<Option<Task<Cheese>>>::new(None))?
        // .add_module(NaoModule)?
        // .add_module(FilterModule)?
        .add_module(AsyncModule)?
        .add_system(send_cheese)
        .add_system(receive_cheese.after(send_cheese))
        .run()?;
    Ok(())
}
