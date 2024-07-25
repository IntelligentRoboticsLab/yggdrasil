use miette::Result;
use std::{collections::HashMap, sync::MutexGuard};

use yggdrasil::core::control::transmit::RobotStateMsg;

#[derive(Default, Debug)]
pub struct RobotResources(pub HashMap<String, String>);

impl RobotResources {
    pub fn update_resources(
        &mut self,
        updated_state_msg: RobotStateMsg,
        focused_resource: MutexGuard<Option<String>>,
    ) -> Result<()> {
        let updated_resource_map = updated_state_msg.0;
        println!("Focussed on resource: {:?}", focused_resource);

        for (name, updated_data) in updated_resource_map.into_iter() {
            if let Some(data) = self.0.get_mut(&name) {
                // Do not update a resource if user is focussed on it
                if let Some(focused_resource) = focused_resource.as_ref() {
                    if name == *focused_resource {
                        continue;
                    }
                }
                *data = updated_data;
            } else {
                self.0.insert(name, updated_data);
            }
        }
        Ok(())
    }
}
