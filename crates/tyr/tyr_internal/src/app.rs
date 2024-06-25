use crate::schedule::{Dependency, DependencySystem, Schedule};
use crate::storage::{Resource, Storage};
use crate::system::{IntoSystem, IntoSystemChain, StartupSystem, System};
use crate::{IntoDependencySystem, Module};

use miette::Result;

/// The glue that binds systems and resources together, and allows them to be executed.
#[derive(Default)]
pub struct App {
    systems: Vec<DependencySystem<()>>,
    storage: Storage,
}

struct ScheduledApp {
    schedule: Schedule,
    storage: Storage,
}

impl App {
    /// Initialize a new app without any systems or resources.
    pub fn new() -> Self {
        Self {
            systems: Vec::new(),
            storage: Storage::new(),
        }
    }

    /// Adds a system to the app.
    ///
    /// The system will be automatically sorted by the scheduler based on the dependencies.
    ///
    /// # Explicit ordering
    /// Explicit ordering of systems can be achieved using the [`IntoDependencySystem`] trait.
    /// This trait allows systems to be declared to run before or after other systems,
    /// which useful when order of execution is important.
    ///
    /// [`.before()`](`IntoDependencySystem::before`) can be used to schedule the specified system before a system.
    ///
    /// [`.after()`](`IntoDependencySystem::after`) can be used to schedule the specified system after a system.
    ///
    /// # Note:
    /// This is the only way to guarantee the execution order of systems. If you do not specify ordering,
    /// two systems may run before or after another, or even in parallel if they do not access the same resources mutably.
    ///
    /// # Example
    /// ```
    /// use miette::Result;
    /// use tyr_internal::*;
    ///
    /// fn main() {
    ///     let app = App::new()
    ///         .add_system(foo_system)
    ///         // this system runs after `foo_system`
    ///         .add_system(bar_system.after(foo_system))
    ///         // this system runs before `bar_system` and after `foo_system`
    ///         .add_system(baz_system.before(bar_system).after(foo_system));
    /// }
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
    pub fn add_system<I>(mut self, system: impl IntoDependencySystem<I>) -> Self {
        self.systems
            // Turns system into `DependencySystem<I>` then transforms it to `DependencySystem<()>`
            .push(system.into_dependency_system().into_dependency_system());
        self
    }

    #[must_use]
    pub fn add_system_with_index<I>(
        mut self,
        system: impl IntoDependencySystem<I>,
        order_index: u8,
    ) -> Self {
        self.systems
            // Turns system into `DependencySystem<I>` then transforms it to `DependencySystem<()>`
            .push(
                system
                    .into_dependency_system_with_index(order_index)
                    .into_dependency_system_with_index(order_index),
            );
        self
    }

    #[must_use]
    /// Adds a chain of systems to the app
    ///
    /// The systems added run sequentially, i.e.:
    /// ```ignore
    /// app.add_system_chain((first, second, third))
    /// ```
    /// is equivalent to
    /// ```ignore
    /// app.add_system(first)
    ///    .add_system(second.after(first))
    ///    .add_system(third.after(second))
    /// ```
    pub fn add_system_chain<I>(self, systems: impl IntoSystemChain<I>) -> Self {
        let mut system_chain = systems.chain();

        for i in 1..system_chain.len() {
            // create a dependency on the previous system
            let prev_system = system_chain[i - 1].boxed_system();
            let dependency = Dependency::After(prev_system.clone());

            system_chain[i].add_dependency(dependency);
        }

        system_chain
            .into_iter()
            .fold(self, |app, system| app.add_system(system))
    }

    /// Adds a startup system to the app.
    ///
    /// A startup system is executed once when the app starts up,
    /// and is provided access to the [`Storage`] of the app.
    ///
    /// All startup systems must be functions with at least the following signature:
    /// ```ignore
    /// #[startup_system]
    /// fn my_startup_system(storage: &mut Storage) -> Result<()>
    /// ```
    /// After the first parameter, you can query any resource `T` by using `&T` or `&mut T` like in a normal system.
    /// ```ignore
    /// #[startup_system]
    /// fn another_startup_system(storage: &mut Storage, foo: &Foo, bar: &Bar) -> Result<()>
    /// ```
    ///
    /// Startup systems can be useful for values that need to be initialized once before they are used.
    ///
    /// # Example
    /// ```ignore
    /// fn get_robot_connection(storage: &mut Storage) -> Result<()> {
    ///     // get some initial connection
    ///     let connection = Robot::connect();
    ///     // and add it to the storage
    ///     storage.add_resource(Resource::new(connection))?;
    ///     // now our connection can be used by all systems.
    ///     Ok(())
    /// }
    /// ```
    pub fn add_startup_system<Input>(
        mut self,
        system: impl IntoSystem<StartupSystem, Input>,
    ) -> Result<Self> {
        system.into_system().run(&mut self.storage)?;
        Ok(self)
    }

    /// Creates a [`Resource<T>`] on `T`s that implement [`Default`] and adds it to the app storage.
    ///
    /// # Errors
    /// This function fails if there is already a resource of type `T` in the storage.
    pub fn init_resource<T: Default + Send + Sync + 'static>(mut self) -> Result<Self> {
        self.storage.add_resource(Resource::<T>::default())?;
        Ok(self)
    }

    /// Consumes the [`Resource<T>`] and adds it to the app storage.
    ///
    /// # Errors
    /// This function fails if there is already a resource of type `T` in the storage.
    pub fn add_resource<T: Send + Sync + 'static>(mut self, res: Resource<T>) -> Result<Self> {
        self.storage.add_resource(res)?;
        Ok(self)
    }

    /// Creates a [`Resource<T>`] on `T`s that implement [`Default`] and adds it to the app storage.
    ///
    /// This method is used to mark [`Resource<T>`] as a debuggable resource, making it
    /// show up in the debug panel.
    ///
    /// # Errors
    /// This function fails if there is already a resource of type `T` in the storage.
    pub fn init_debuggable_resource<T: std::fmt::Debug + Default + Send + Sync + 'static>(
        mut self,
    ) -> Result<Self> {
        self.storage
            .add_debuggable_resource(Resource::<T>::default())?;
        Ok(self)
    }

    /// Consumes the [`Resource<T>`] and adds it to the app storage.
    ///
    /// This method is used to mark [`Resource<T>`] as a debuggable resource, making it
    /// show up in the debug panel.
    ///
    /// # Errors
    /// This functions fails if there is already a resource of type `T` in the storage.
    pub fn add_debuggable_resource<T: std::fmt::Debug + Send + Sync + 'static>(
        mut self,
        res: Resource<T>,
    ) -> Result<Self> {
        self.storage.add_debuggable_resource(res)?;
        Ok(self)
    }

    /// Consumes the [`Module`] and incorporates it into the app.
    /// The module must implement the [`Module`] trait, which defines the [`.initialize()`](`Module::initialize`) method.
    /// The [`.initialize()`](`Module::initialize`) method allows the [`Module`] to add resource and systems to the app.
    pub fn add_module<T: Module>(self, module: T) -> Result<Self> {
        module.initialize(self)
    }

    /// Creates a schedule from the specified app structure and executes it.
    #[must_use = "Scheduled app should be used!"]
    pub fn run(self) -> Result<()> {
        let mut app = ScheduledApp {
            schedule: Schedule::with_dependency_systems(self.systems)?,
            storage: self.storage,
        };

        app.run()
    }
}

impl ScheduledApp {
    fn run(&mut self) -> Result<()> {
        self.schedule.check_ordered_dependencies()?;
        self.schedule.build_graph()?;

        loop {
            self.schedule.execute(&mut self.storage)?;
        }
    }
}
