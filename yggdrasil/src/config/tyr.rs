use crate::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TyrConfig {
    tasks: tyr::tasks::TaskConfig,
}

impl Config for TyrConfig {
    const PATH: &'static str = "tyr.toml";
}

// TODO: this is not okay
// Ideally we want to directly implement `Config` for the `TaskConfig` instead of using a wrapper and hacking in the correct resource like this. However, this requires a refactor of odal/tyr/yggdrasil for which we currently have no time :D
#[startup_system]
pub(super) fn configure_tyr_hack(storage: &mut Storage, tyr_config: &TyrConfig) -> Result<()> {
    storage.add_resource(Resource::new(tyr_config.tasks.clone()))
}
