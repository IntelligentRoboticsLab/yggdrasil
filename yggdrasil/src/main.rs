use tyr::{prelude::*, tasks::TaskModule};

use miette::Result;

use yggdrasil::{filter::FilterModule, nao::NaoModule};

fn main() -> Result<()> {
    tracing_subscriber::fmt::fmt().pretty().init();

    miette::set_panic_hook();

    App::new()
        .add_module(TaskModule)?
        .add_module(NaoModule)?
        .add_module(FilterModule)?
        .run()?;
    Ok(())
}
