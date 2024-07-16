use std::path::Path;

use crate::schedule::{Dependency, DependencySystem, Schedule};
use crate::storage::{Resource, Storage};
use crate::system::{IntoSystem, IntoSystemChain, StartupSystem, System};
use crate::{ControlSocket, Inspect, IntoDependencySystem, Module};

use miette::{IntoDiagnostic, Result};

#[derive(Debug, Default, Clone, Copy)]
pub enum SystemStage {
    /// This stage runs directly after the previous data has been sent to the LoLA socket.
    ///
    /// This stage is run first, before the main execution loop.
    Init,
    /// This stage is used to update resources that depend on sensor data.
    ///
    /// This stage runs: *After* [`SystemStage::Init`], and *before* [`SystemStage::Execute`].
    Sensor,
    /// This stage is used for the main execution loop, and is where most systems will run.
    ///
    /// This stage runs: *After* [`SystemStage::Sensor`], and *before* [`SystemStage::Finalize`].
    #[default]
    Execute,
    /// This stage runs at the end of the main execution, finalizing resources before
    /// they are written to the LoLA socket.
    ///
    /// This stage runs: *After* [`SystemStage::Execute`], and *before* [`SystemStage::Write`].
    Finalize,
    /// This stage runs while the data is being written to the LoLA socket.
    ///
    /// This stage is used for systems that interact with the LoLA socket, or depend on the write order.
    Write,
    /// This stage runs after the data has been updated from the LoLA socket, and is used for systems
    /// that depend on the most up-to-date data.
    ///
    /// It differs from [`SystemStage::Init`] in the sense that, systems are still classified as
    /// running in the current cycle.
    ///
    /// This stage runs *after* [`SystemStage::Write`].
    PostWrite,
    /// A custom stage that can be used for any purpose.
    ///
    /// **This should be used sparingly, as it can make the execution order less clear.**
    Custom(u8),
}

impl SystemStage {
    pub fn index(&self) -> u8 {
        match self {
            SystemStage::Init => 20,
            SystemStage::Sensor => 50,
            SystemStage::Execute => (256u32 / 2u32) as u8,
            SystemStage::Finalize => 140,
            SystemStage::Write => 200,
            SystemStage::PostWrite => 240,
            SystemStage::Custom(value) => *value,
        }
    }
}

/// The glue that binds systems and resources together, and allows them to be executed.
#[derive(Default)]
pub struct App {
    systems: Vec<DependencySystem<()>>,
    startup_systems: Vec<Box<dyn System<StartupSystem>>>,
    storage: Storage,
}

impl App {
    /// Initialize a new app without any systems or resources.
    pub fn new() -> Self {
        Self {
            systems: Vec::new(),
            startup_systems: Vec::new(),
            storage: Storage::new(),
        }
    }

    #[must_use]
    /// Add a system to the app.
    ///
    /// Same as [`add_staged_system`](App::add_staged_system) with [`SystemStage::Execute`].
    pub fn add_system<I>(self, system: impl IntoDependencySystem<I>) -> Self {
        self.add_staged_system(SystemStage::Execute, system)
    }

    /// Add a system to the app with the specified stage.
    ///
    /// Stages are executed in ascending order of their [`index`](SystemStage::index).
    /// The systems in the same stage will be automatically sorted by the scheduler based on the dependencies.
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
    pub fn add_staged_system<I>(
        mut self,
        stage: SystemStage,
        system: impl IntoDependencySystem<I>,
    ) -> Self {
        self.systems
            // Turns system into `DependencySystem<I>` then transforms it to `DependencySystem<()>`
            .push(
                system
                    .into_dependency_system_with_index(stage.index())
                    .into_dependency_system_with_index(stage.index()),
            );
        self
    }

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
    #[must_use]
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

    /// Adds a chain of systems to the app, with the specified stage.
    ///
    /// The systems added run sequentially, i.e.:
    /// ```ignore
    /// app.add_staged_system_chain(SystemStage::Sensor, (first, second, third))
    /// ```
    /// is equivalent to
    /// ```ignore
    /// app.add_staged_system(SystemStage::Sensor, first)
    ///    .add_staged_system(SystemStage::Sensor, second.after(first))
    ///    .add_staged_system(SystemStage::Sensor, third.after(second))
    /// ```
    #[must_use]
    pub fn add_staged_system_chain<I>(
        self,
        stage: SystemStage,
        systems: impl IntoSystemChain<I>,
    ) -> Self {
        let mut system_chain = systems.chain();

        for i in 1..system_chain.len() {
            // create a dependency on the previous system
            let prev_system = system_chain[i - 1].boxed_system();
            let dependency = Dependency::After(prev_system.clone());

            system_chain[i].add_dependency(dependency);
        }

        system_chain
            .into_iter()
            .fold(self, |app, system| app.add_staged_system(stage, system))
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
        self.startup_systems.push(Box::new(system.into_system()));

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
    /// This method is used to mark [`Resource<T>`] as a inspectable resource, making it
    /// show up in the debug panel.
    ///
    /// # Errors
    /// This function fails if there is already a resource of type `T` in the storage.
    pub fn init_inspectable_resource<T: Inspect + Default + Send + Sync + 'static>(
        mut self,
    ) -> Result<Self> {
        self.storage
            .add_inspectable_resource(Resource::<T>::default())?;
        Ok(self)
    }

    /// Consumes the [`Resource<T>`] and adds it to the app storage.
    ///
    /// This method is used to mark [`Resource<T>`] as a inspectable resource, making it
    /// show up in the debug panel.
    ///
    /// # Errors
    /// This functions fails if there is already a resource of type `T` in the storage.
    pub fn add_inspectable_resource<T: Inspect + Send + Sync + 'static>(
        mut self,
        res: Resource<T>,
    ) -> Result<Self> {
        self.storage.add_inspectable_resource(res)?;
        Ok(self)
    }

    /// Consumes the [`Module`] and incorporates it into the app.
    /// The module must implement the [`Module`] trait, which defines the [`.initialize()`](`Module::initialize`) method.
    /// The [`.initialize()`](`Module::initialize`) method allows the [`Module`] to add resource and systems to the app.
    pub fn add_module<T: Module>(self, module: T) -> Result<Self> {
        module.initialize(self)
    }

    fn run_startup_systems(&mut self) -> Result<()> {
        for startup_system in &mut self.startup_systems {
            startup_system.run(&mut self.storage)?;
        }

        Ok(())
    }

    /// Creates a schedule from the specified app structure and executes it.
    #[must_use = "Scheduled app should be used!"]
    pub fn run(mut self) -> Result<()> {
        self.run_startup_systems()?;
        ScheduledApp::new(self)?.run()
    }

    /// Store a dependency graph of all systems as a png.
    ///
    /// The dependency graph shows which systems depend on other systems.
    /// Dependencies are created using [`before`](IntoDependencySystem::before) and
    /// [`after`](IntoDependencySystem::after).
    ///
    /// ## Example
    /// ```
    /// # use tyr_internal::App;
    /// # fn example(app: &App) {
    /// app.store_system_dependency_graph("../dependency_graph.png");
    /// # }
    /// ```
    pub fn store_system_dependency_graph<P>(&self, path: P) -> Result<()>
    where
        P: AsRef<Path>,
    {
        let mut schedule = Schedule::with_dependency_systems(self.systems.clone())?;
        schedule.check_ordered_dependencies()?;
        schedule.build_graph()?;
        schedule.generate_graph(path)
    }
}

struct ScheduledApp {
    schedule: Schedule,
    storage: Storage,
    socket: ControlSocket,
}

impl ScheduledApp {
    fn new(app: App) -> Result<Self> {
        Ok(Self {
            schedule: Schedule::with_dependency_systems(app.systems)?,
            storage: app.storage,
            socket: ControlSocket::new().into_diagnostic()?,
        })
    }

    fn run(&mut self) -> Result<()> {
        self.schedule.check_ordered_dependencies()?;
        self.schedule.build_graph()?;

        loop {
            self.schedule.execute(&mut self.storage)?;
            self.storage
                .map_resource_mut(|view| self.socket.tick(&mut self.schedule, view))?
                .into_diagnostic()?;
        }
    }
}
