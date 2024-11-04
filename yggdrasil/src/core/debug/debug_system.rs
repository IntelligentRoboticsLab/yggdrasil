use std::collections::HashMap;

use bevy::{ecs::schedule::ScheduleLabel, prelude::*};
use serde::{Deserialize, Serialize};

#[derive(Resource, Serialize, Deserialize, Debug, Default, Clone)]
pub struct DebugEnabledSystems {
    pub systems: HashMap<String, bool>,
}

impl DebugEnabledSystems {
    pub fn set_system(&mut self, name: String, enabled: bool) {
        if let Some(current_enabled) = self.systems.get_mut(&name) {
            *current_enabled = enabled;
        } else {
            tracing::error!("System `{}` does not exist", name);
        }
    }
}

pub fn debug_enabled(system_name: impl ToString) -> impl Condition<()> {
    let name = system_name.to_string();

    IntoSystem::into_system(move |enabled: Res<DebugEnabledSystems>| {
        enabled.systems.get(&name).copied().unwrap_or(false)
    })
}

pub trait DebugAppExt {
    fn add_debug_systems<M>(
        &mut self,
        schedule: impl ScheduleLabel,
        systems: impl IntoSystemConfigs<M>,
    ) -> &mut Self;

    fn add_named_debug_systems<M>(
        &mut self,
        schedule: impl ScheduleLabel,
        systems: impl IntoSystemConfigs<M>,
        systems_name: impl ToString,
    ) -> &mut Self;
}

impl DebugAppExt for App {
    fn add_debug_systems<M>(
        &mut self,
        schedule: impl ScheduleLabel,
        systems: impl IntoSystemConfigs<M>,
    ) -> &mut Self {
        let system_name = std::any::type_name_of_val(&systems);
        self.add_named_debug_systems(schedule, systems, system_name.to_string())
    }

    fn add_named_debug_systems<M>(
        &mut self,
        schedule: impl ScheduleLabel,
        systems: impl IntoSystemConfigs<M>,
        systems_name: impl ToString,
    ) -> &mut Self {
        let world = self.world_mut();
        let mut debug_enabled_systems = world.resource_mut::<DebugEnabledSystems>();
        debug_enabled_systems
            .systems
            .insert(systems_name.to_string(), true);
        self.add_systems(schedule, systems.run_if(debug_enabled(systems_name)))
    }
}
