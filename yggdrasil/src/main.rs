pub mod r#async;
pub mod event;
pub mod filter;
pub mod nao;

use std::{any::type_name, time::Duration};

use event::Event;
// use filter::FilterModule;
use miette::Result;
// use nao::NaoModule;
use r#async::{
    runtime::{AsyncDispatcher, AsyncTask},
    AsyncModule,
};
use tyr::prelude::*;

#[derive(Debug)]
struct Cheese;

#[system]
fn send_cheese(dispatcher: &mut AsyncDispatcher, task: &mut AsyncTask<Cheese>) -> Result<()> {
    // Task is already active
    if task.is_alive() {
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

    task.spawn(dispatcher.dispatch(get_cheese()));

    Ok(())
}

#[system]
fn store_async_on_completion<T: Send + Sync + 'static>(
    task: &mut AsyncTask<T>,
    resource: &mut T,
) -> Result<()> {
    if let Some(result) = task.poll() {
        *resource = result;
        println!(
            "Completed async task and stored resource of type `{}`",
            type_name::<T>()
        );
        task.kill();
    }

    Ok(())
}

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    App::new()
        // TODO: Some kind of task/general event storage apart from resources?
        .add_resource(Resource::<AsyncTask<Cheese>>::default())?
        .add_resource(Resource::new(Cheese))?
        // .add_module(NaoModule)?
        // .add_module(FilterModule)?
        .add_module(AsyncModule)?
        .add_system(send_cheese)
        .add_system(store_async_on_completion::<Cheese>.after(send_cheese))
        .run()?;
    Ok(())
}
