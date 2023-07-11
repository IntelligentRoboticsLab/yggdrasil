use crate::schedule::{Schedule, SystemOrdering};
use crate::storage::{Resource, Storage};
use crate::{IntoSystemOrdering, Module};

use color_eyre::Result;

#[derive(Default)]
pub struct App {
    systems: Vec<SystemOrdering<()>>,
    storage: Storage,
}

pub struct ScheduledApp {
    schedule: Schedule,
    storage: Storage,
}

impl App {
    pub fn new() -> Self {
        Self {
            systems: Vec::new(),
            storage: Storage::new(),
        }
    }

    #[must_use]
    pub fn add_system<I>(mut self, system: impl IntoSystemOrdering<I>) -> Self {
        self.systems
            // Turns system into `SystemOrdering<I>` then transforms it to `SystemOrdering<()>`
            .push(system.into_system_ordering().into_system_ordering());
        self
    }

    pub fn add_startup_system<F: FnOnce(&mut Storage) -> Result<()>>(
        mut self,
        system: F,
    ) -> Result<Self> {
        system(&mut self.storage)?;
        Ok(self)
    }

    /// Consumes the [`Resource<T>`] and adds it to app storage by turning it into a storable [`crate::storage::ErasedResource`]
    pub fn add_resource<T: Send + Sync + 'static>(mut self, res: Resource<T>) -> Result<Self> {
        self.storage.add_resource(res)?;
        Ok(self)
    }

    pub fn add_module<T: Module>(self, module: T) -> Result<Self> {
        module.build(self)
    }

    /// Consumes self to construct a DAG from the specified structure and makes the app runnable
    #[must_use]
    pub fn build(self) -> Result<ScheduledApp> {
        Ok(ScheduledApp {
            schedule: Schedule::with_system_orderings(self.systems)?,
            storage: self.storage,
        })
    }
}

impl ScheduledApp {
    pub fn run(&mut self) -> Result<()> {
        self.schedule.execute(&mut self.storage)?;
        Ok(())
    }
}
