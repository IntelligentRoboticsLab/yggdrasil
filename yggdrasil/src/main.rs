pub mod behaviour;
pub mod filter;
pub mod nao;

use behaviour::BehaviourModule;
use filter::FilterModule;
use miette::Result;
use nao::NaoModule;
use tyr::{prelude::*, tasks::TaskModule};

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    miette::set_panic_hook();

    App::new()
        .add_module(TaskModule)?
        .add_module(NaoModule)?
        .add_module(FilterModule)?
        .add_module(BehaviourModule)?
        .run()?;
    Ok(())
}
