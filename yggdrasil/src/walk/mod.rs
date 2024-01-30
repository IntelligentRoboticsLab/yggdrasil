pub mod engine;
pub mod smoothing;
mod states;

use std::{
    ops::Add,
    time::{Duration, Instant},
};

use miette::Result;

use tyr::prelude::*;

use crate::{filter, nao};

#[derive(Default, Debug, Clone)]
pub struct Odometry {
    pub forward: f32,
    pub left: f32,
    pub turn: f32,
}

impl Add for Odometry {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            forward: self.forward + rhs.forward,
            left: self.left + rhs.left,
            turn: self.turn + rhs.turn,
        }
    }
}

pub struct WalkingEngineModule;

impl Module for WalkingEngineModule {
    fn initialize(self, app: App) -> Result<App> {
        Ok(app
            .add_startup_system(initialize_cycle_counter)?
            .add_system(update_cycle_time.after(nao::write_hardware_info))
            .add_resource(Resource::new(engine::WalkingEngine::default()))?
            .init_resource::<Odometry>()?
            .add_system(
                engine::walking_engine
                    .after(update_cycle_time)
                    .after(filter::fsr::force_sensitive_resistor_filter)
                    .after(filter::imu::imu_filter),
            )
            .add_system(engine::toggle_walking_engine.before(engine::walking_engine)))
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
