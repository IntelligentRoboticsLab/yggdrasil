pub mod filter;
pub mod nao;

use filter::FilterModule;
use miette::Result;
use nao::NaoModule;
use tyr::prelude::*;

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    App::new()
        .add_module(NaoModule)?
        .add_module(FilterModule)?
        .run()?;
    Ok(())
}
