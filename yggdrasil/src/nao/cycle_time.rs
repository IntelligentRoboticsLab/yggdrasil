use std::time::{Duration, Instant};

use crate::prelude::*;

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
    storage.add_resource(Resource::new(CycleTime {
        cycle_start: Instant::now(),
        duration: Duration::from_secs(0),
    }))
}

#[system]
pub fn update_cycle_time(cycle_time: &mut CycleTime) -> Result<()> {
    cycle_time.duration = Instant::now().duration_since(cycle_time.cycle_start);
    cycle_time.cycle_start = Instant::now();

    Ok(())
}
