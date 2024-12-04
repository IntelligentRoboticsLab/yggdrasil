use std::time::{Duration, Instant};

use crate::prelude::*;
use bevy::prelude::*;

/// Plugin that adds resources and systems for tracking the cycle time of yggdrasil.
pub(super) struct CycleTimePlugin;

impl Plugin for CycleTimePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostStartup, initialize_cycle_counter);
        app.add_systems(PostWrite, update_cycle_stats);
    }
}

/// A resource that keeps track of the number of cycles since yggdrasil has been running.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Resource, Component)]
pub struct Cycle(pub usize);

/// A resource that keeps track of the time it takes to complete a full cycle of the yggdrasil framework.
///
/// This should always be around 11-12ms, as the hardware runs at around 83Hz. However a slow system might result in a higher cycle time.
#[derive(Resource, Debug)]
pub struct CycleTime {
    pub cycle_start: Instant,
    pub duration: Duration,
}

pub(crate) fn initialize_cycle_counter(mut commands: Commands) {
    commands.insert_resource(Cycle::default());
    commands.insert_resource(CycleTime {
        cycle_start: Instant::now(),
        duration: Duration::ZERO,
    });
}

fn update_cycle_stats(mut cycle: ResMut<Cycle>, mut cycle_time: ResMut<CycleTime>) {
    cycle.0 += 1;
    cycle_time.duration = Instant::now().duration_since(cycle_time.cycle_start);
    cycle_time.cycle_start = Instant::now();
}
