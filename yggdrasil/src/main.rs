use miette::Result;
use tyr::{prelude::*, tasks::TaskModule};

pub mod behaviour;
pub mod filter;
pub mod leds;
pub mod nao;

use behaviour::BehaviourModule;
use filter::FilterModule;
use leds::LedsModule;
use nao::NaoModule;

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    miette::set_panic_hook();

    App::new()
        .add_module(BehaviourModule)?
        .add_module(FilterModule)?
        .add_module(LedsModule)?
        .add_module(NaoModule)?
        .add_module(TaskModule)?
        .run()?;
    Ok(())
}
