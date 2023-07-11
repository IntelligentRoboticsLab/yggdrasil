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
    /// Initialize a new [`App`].
    pub fn new() -> Self {
        Self {
            systems: Vec::new(),
            storage: Storage::new(),
        }
    }

    /// Adds a system to the [`App`].
    ///
    /// The system will be automatically sorted by the scheduler based on the dependencies.
    ///
    /// # Explicit ordering
    /// Explicit ordering of systems can be achieved using the [`IntoSystemOrdering`] trait.
    /// This trait allows systems to be declared before or after other systems, useful when order is important.
    ///
    /// [`IntoSystemOrdering::after`] can be used to schedule the specified system after a system.
    ///
    /// [`IntoSystemOrdering::before`] can be used to schedule the specified system before a system.
    ///
    /// ## Example
    /// ```no_run
    /// use color_eyre::Result;
    /// use crate::tyr_internal::{App, IntoSystemOrdering};
    ///
    /// App::new().add_system(foo_system.after(bar_system).before(baz_system));
    ///
    /// fn foo_system() -> Result<()> {
    ///     Ok(())
    /// }
    ///
    /// fn bar_system() -> Result<()> {
    ///     Ok(())
    /// }
    ///
    /// fn baz_system() -> Result<()> {
    ///     Ok(())
    /// }
    /// ```
    #[must_use]
    pub fn add_system<I>(mut self, system: impl IntoSystemOrdering<I>) -> Self {
        self.systems
            // Turns system into `SystemOrdering<I>` then transforms it to `SystemOrdering<()>`
            .push(system.into_system_ordering().into_system_ordering());
        self
    }

    /// Adds a startup system to the [`App`].
    ///
    /// The startup system is executed once, when the [`App`] starts up, and is provided access to the [`Storage`] of the [`App`].
    pub fn add_startup_system<F: FnOnce(&mut Storage) -> Result<()>>(
        mut self,
        system: F,
    ) -> Result<Self> {
        system(&mut self.storage)?;
        Ok(self)
    }

    /// Consumes the [`Resource<T>`] and adds it to [`App`] storage by turning it into a storable [`crate::storage::ErasedResource`]
    pub fn add_resource<T: Send + Sync + 'static>(mut self, res: Resource<T>) -> Result<Self> {
        self.storage.add_resource(res)?;
        Ok(self)
    }

    /// Consumes the [`Module`] and incorporates it into the [`App`].
    /// The module must implement the `Module` trait, which defines the `build` method.
    /// The [`Module::initialize`] allows the [`Module`] to add resource and systems to the [`App`].
    pub fn add_module<T: Module>(self, module: T) -> Result<Self> {
        module.initialize(self)
    }

    /// Consumes self to construct a DAG from the specified structure and makes the [`App`] runnable.
    #[must_use = "Scheduled app should be used!"]
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
