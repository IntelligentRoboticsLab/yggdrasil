use bevy::{
    app::MainScheduleOrder,
    ecs::schedule::{LogLevel, ScheduleBuildSettings, ScheduleLabel},
    prelude::*,
};

/// The schedule that contains logic that updates resources using sensor data.
///
/// This schedule runs directly after the [`First`] schedule, and is used to update resources
/// that depend on sensor data.
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Sensor;

/// The schedule that runs before [`Write`].
///
/// For example this is used to finalize any changes in the [`super::nao::NaoManager`]
/// and update the control messages that will be sent to the `LoLA` socket.
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct PreWrite;

/// The schedule runs logic required to read and write data to the `LoLA` socket.
///
/// This stage is used for systems that interact with the `LoLA` socket, or depend on the write order.
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Write;

/// This stage runs after the data has been written to the `LoLA` socket, and is used for systems
/// that depend on the most up-to-date data.
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct PostWrite;

/// Plugin configures the robot specific schedules in the [`MainScheduleOrder`].
pub struct NaoSchedulePlugin;

impl Plugin for NaoSchedulePlugin {
    fn build(&self, app: &mut App) {
        // app.edit_schedule(Update, |schedule| {
        //     schedule.set_build_settings(ScheduleBuildSettings {
        //         ambiguity_detection: LogLevel::Warn,
        //         ..default()
        //     });
        // });

        // Add the custom schedules to the main schedule.
        app.world_mut()
            .resource_scope(|_, mut schedule: Mut<MainScheduleOrder>| {
                schedule.insert_after(First, Sensor);
                schedule.insert_after(PostUpdate, PreWrite);
                schedule.insert_after(PreWrite, Write);
                schedule.insert_after(Write, PostWrite);
            });
    }
}
