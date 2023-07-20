pub mod dnt_walk;
pub mod engine;
pub mod kinematics;

use std::time::{Duration, Instant};

use miette::Result;

use tyr::prelude::*;

use crate::nao;

use self::engine::WalkingEngine;

pub struct WalkingEngineModule;

impl Module for WalkingEngineModule {
    fn initialize(self, app: App) -> Result<App> {
        Ok(app
            .add_startup_system(initialize_cycle_counter)?
            .add_system(update_cycle_time.after(nao::write_hardware_info))
            .add_resource(Resource::new(dnt_walk::WalkingEngine::default()))?
            .add_system(
                dnt_walk::walking_engine
                    .after(update_cycle_time)
                    .after(super::filter::fsr::force_sensitive_resistor_filter)
                    .after(super::filter::imu::imu_filter),
            )
            .add_system(dnt_walk::toggle_walking_engine.before(dnt_walk::walking_engine))
            .add_system(
                crate::framework::filter::imu::fallingstate.after(dnt_walk::walking_engine),
            ))
        // .add_resource(Resource::new(WalkingEngine::default()))?
        // .add_system(engine::walking_engine.after(nao::write_hardware_info))
        // .add_system(engine::toggle_walking_engine.before(engine::walking_engine)))
    }
}

#[derive(Debug)]
pub struct CycleTime {
    pub cycle_start: Instant,
    pub duration: Duration,
}

fn initialize_cycle_counter(storage: &mut Storage) -> Result<()> {
    storage.add_resource(Resource::new(CycleTime {
        cycle_start: Instant::now(),
        duration: Duration::from_secs(0),
    }))
}

#[system]
fn update_cycle_time(cycle_time: &mut CycleTime) -> Result<()> {
    cycle_time.duration = Instant::now().duration_since(cycle_time.cycle_start);
    cycle_time.cycle_start = Instant::now();
    // info!("cycle_time: {}ms", cycle_time.duration.as_millis());

    Ok(())
}
