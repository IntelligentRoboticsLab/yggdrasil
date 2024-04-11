use std::time::{Duration, Instant};

use crate::prelude::*;

/// A resource that keeps track of the number of cycles since yggdrasil has been running.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cycle(pub usize);

/// A resource that keeps track of the time it takes to complete a full cycle of the yggdrasil framework.
///
/// This should always be around 11-12ms, as the hardware runs at around 83Hz. However a slow system might result in a higher cycle time.
#[derive(Debug)]
pub struct CycleTime {
    pub cycle_start: Instant,
    pub duration: Duration,
}

#[startup_system]
pub(crate) fn initialize_cycle_counter(storage: &mut Storage) -> Result<()> {
    storage.add_resource(Resource::new(Cycle::default()))?;
    storage.add_resource(Resource::new(CycleTime {
        cycle_start: Instant::now(),
        duration: Duration::ZERO,
    }))
}

#[system]
pub fn update_cycle_stats(cycle: &mut Cycle, cycle_time: &mut CycleTime) -> Result<()> {
    cycle.0 += 1;
    cycle_time.duration = Instant::now().duration_since(cycle_time.cycle_start);
    cycle_time.cycle_start = Instant::now();

    Ok(())
}
