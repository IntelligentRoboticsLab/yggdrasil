use miette::Result;
use tyr::prelude::*;

use self::damage_prevention::DamagePreventionModule;

pub mod damage_prevention;

pub struct BehaviourModule;

impl Module for BehaviourModule {
    fn initialize(self, app: App) -> Result<App> {
        app.add_module(DamagePreventionModule)
    }
}
