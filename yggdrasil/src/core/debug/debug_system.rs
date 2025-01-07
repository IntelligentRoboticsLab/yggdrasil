use bevy::{ecs::schedule::ScheduleLabel, prelude::*};
use re_control_comms::debug_system::DebugEnabledSystems;

/// Run condition for a specified system (using the system name) based on
/// the corresponding flag for that systems, stored in the resource
/// [`DebugEnabledSystems`]
fn debug_enabled(system_name: impl ToString) -> impl Condition<()> {
    let name = system_name.to_string();

    // Create a system to check the enabled flag for the specified system
    IntoSystem::into_system(move |enabled: Res<DebugEnabledSystems>| {
        enabled.systems.get(&name).copied().unwrap_or(false)
    })
}

/// The trait `DebugAppExt` gives wrapper functions around the bevy `App`.
///
/// Instead of only adding system to the `App`, it also adds:
/// - Add the system name to the resource [`DebugEnabledSystems`]
/// - Add the run condition `debug_enabled` to the system. This will make it
///   that this system will only run when it is flagged as enabled in the
///   [`DebugEnabledSystems`]
///
/// ## Example
/// ```no_run
/// use bevy::prelude::*;
///
/// struct CustomPlugin
///
/// impl Plugin for CustomPlugin {
///     fn build(&self, app: &mut App) {
///         app.add_debug_systems(Update, wee_sound_system)
///         .add_named_debug_systems(
///             PostUpdate,
///             visualize_lines::<Top>.run_if(resource_exists_and_changed::<DetectedLines<Top>>),
///             "Visualize lines",
///         );
///     }
/// }
/// ```
///
pub trait DebugAppExt {
    /// Add a system to the schedule and add the system name to the
    /// [`DebugEnabledSystems`] resource. The system will only run when the
    /// flag for this system is enabled in the [`DebugEnabledSystems`].
    fn add_debug_systems<M>(
        &mut self,
        schedule: impl ScheduleLabel,
        systems: impl IntoSystemConfigs<M>,
        enabled: bool,
    ) -> &mut Self;

    /// Add a system to the schedule and add the system name to the
    /// [`DebugEnabledSystems`] resource. The system will only run when the
    /// flag for this system is enabled in the [`DebugEnabledSystems`].
    fn add_named_debug_systems<M>(
        &mut self,
        schedule: impl ScheduleLabel,
        systems: impl IntoSystemConfigs<M>,
        systems_name: impl ToString,
        enabled: bool,
    ) -> &mut Self;
}

impl DebugAppExt for App {
    fn add_debug_systems<M>(
        &mut self,
        schedule: impl ScheduleLabel,
        systems: impl IntoSystemConfigs<M>,
        enabled: bool,
    ) -> &mut Self {
        let system_name = std::any::type_name_of_val(&systems);
        self.add_named_debug_systems(schedule, systems, system_name.to_string(), enabled)
    }

    fn add_named_debug_systems<M>(
        &mut self,
        schedule: impl ScheduleLabel,
        systems: impl IntoSystemConfigs<M>,
        systems_name: impl ToString,
        enabled: bool,
    ) -> &mut Self {
        let world = self.world_mut();
        let mut debug_enabled_systems = world.resource_mut::<DebugEnabledSystems>();
        debug_enabled_systems
            .systems
            .insert(systems_name.to_string(), enabled);
        self.add_systems(schedule, systems.run_if(debug_enabled(systems_name)))
    }
}
