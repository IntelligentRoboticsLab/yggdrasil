use miette::Result;
use tyr::prelude::*;

use super::imu::IMUValues;

pub struct Fall;

impl Module for Fall {
    fn initialize(self, app: App) -> Result<App> {
        Ok(app.add_system(am_i_falling))
    }
}

#[system]
pub fn am_i_falling(imu: &IMUValues) -> Result<()> {
    println!("AM I FALLING YET?! {:?}", imu.gyroscope);
    Ok(())
}