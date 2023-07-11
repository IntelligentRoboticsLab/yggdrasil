pub mod nao;
use color_eyre::Result;
use nao::NaoModule;
use tyr::App;

fn main() -> Result<()> {
    color_eyre::install()?;
    tracing_subscriber::fmt::init();

    App::new().add_module(NaoModule)?.build()?.run()?;

    Ok(())
}
