use tyr::{prelude::*, tasks::TaskModule};

use miette::Result;

use yggdrasil::websocket::WebSocketModule;

fn main() -> Result<()> {
    tracing_subscriber::fmt::fmt().pretty().init();

    miette::set_panic_hook();

    App::new()
        .add_module(TaskModule)?
        // .add_module(NaoModule)?
        // .add_module(FilterModule)?
        .add_module(WebSocketModule)?
        .run()?;
    Ok(())
}
