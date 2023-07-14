pub mod filter;
pub mod nao;

use color_eyre::Result;
use filter::button::ButtonFilter;
use nao::NaoModule;
use tyr::prelude::*;

fn main() -> Result<()> {
    color_eyre::install()?;
    tracing_subscriber::fmt::init();

    App::new()
        .add_module(NaoModule)?
        .add_module(ButtonFilter)?
        .run()?;

    Ok(())
}
