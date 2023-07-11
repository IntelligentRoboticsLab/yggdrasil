use crate::App;
use color_eyre::Result;

pub trait Module {
    fn build(self, app: App) -> Result<App>;
}
