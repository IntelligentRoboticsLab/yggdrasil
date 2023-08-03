pub mod filter;
pub mod motion;
pub mod nao;

use filter::FilterModule;
use miette::Result;
use motion::MotionModule;
use nao::NaoModule;
use tyr::prelude::*;

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    App::new()
        .add_module(NaoModule)?
        .add_module(MotionModule)?
        .add_module(FilterModule)?
        .run()?;
    Ok(())
}
