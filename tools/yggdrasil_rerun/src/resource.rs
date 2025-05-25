use miette::Result;
use std::collections::HashMap;

#[derive(Default, Debug)]
pub struct RobotResources(pub HashMap<String, String>);

impl RobotResources {
    pub fn update_resources(
        &mut self,
        new_resources: HashMap<String, String>,
        changed_resources: &mut HashMap<String, bool>,
    ) -> Result<()> {
        for (resource_name, updated_data) in new_resources.into_iter() {
            if let Some(data) = self.0.get_mut(&resource_name) {
                // Do not update a resource if it is being changed by the user in rerun control
                if let Some(changed_resource) = changed_resources.get(&resource_name) {
                    if *changed_resource {
                        continue;
                    }
                }
                *data = updated_data
            } else {
                self.0.insert(resource_name.clone(), updated_data);
                changed_resources.insert(resource_name, false);
            }
        }
        Ok(())
    }
}
