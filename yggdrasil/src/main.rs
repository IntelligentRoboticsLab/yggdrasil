pub mod filter;
pub mod nao;
pub mod websocket;

use filter::FilterModule;
use miette::Result;
use nao::NaoModule;
use tyr::{prelude::*, tasks::TaskModule};
use websocket::WebsocketModule;

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    miette::set_panic_hook();

    App::new()
        .add_module(TaskModule)?
        .add_module(NaoModule)?
        .add_module(FilterModule)?
        .add_module(WebsocketModule)?
        .run()?;
    Ok(())
}
