use miette::{IntoDiagnostic, Result};
use serde_json::Value;
use std::{collections::HashMap, sync::MutexGuard};

use yggdrasil::core::control::transmit::ControlHostMessage;

#[derive(Default, Debug)]
pub struct RobotResources(pub HashMap<String, String>);

impl RobotResources {
    pub fn update_resources(
        &mut self,
        updated_state_msg: ControlHostMessage,
        mut changed_resources: MutexGuard<HashMap<String, bool>>,
    ) -> Result<()> {
        let updated_resource_map = updated_state_msg.resources;

        for (resource_name, updated_data) in updated_resource_map.into_iter() {
            if let Some(data) = self.0.get_mut(&resource_name) {
                // Do not update a resource if it is being changed by the user in seidr
                if let Some(changed_resource) = changed_resources.get(&resource_name) {
                    if *changed_resource {
                        continue;
                    }
                }
                let updated_data_json: Value =
                    serde_json::from_str(&updated_data).into_diagnostic()?;
                let pretty_updated_data =
                    serde_json::to_string_pretty(&updated_data_json).into_diagnostic()?;
                *data = pretty_updated_data;
            } else {
                self.0.insert(resource_name.clone(), updated_data);
                changed_resources.insert(resource_name, false);
            }
        }
        Ok(())
    }
}
