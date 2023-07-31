use crate::App;
use miette::Result;

/// A module represents a collection of resources and systems that can be added to an application [`App`].
///
/// Modules encapsulate related functionality and provide a way to organize and modularize the codebase.
///
/// They define an [`initialize`](`Module::initialize`) method, which is responsible for adding the required resources and systems
/// to the provided [`App`].
///
/// # Example
///
/// ```
/// use miette::Result;
/// use tyr_internal::*;
///
/// // Define a module struct
/// struct FooModule;
///
/// // Implement the `Module` trait
/// impl Module for FooModule {
///     fn initialize(self, app: App) -> Result<App> {
///         // Add a resource and system to the application
///         Ok(app
///             .add_resource(Resource::new(42_i32))?
///             .add_system(foo_system))
///     }
/// }
///
/// // Define a system function
/// fn foo_system() -> Result<()> {
///     // System logic goes here
///     Ok(())
/// }
///
/// ```
///
/// In this example, the `FooModule` adds a resource `42` and a system `foo_system`
/// to the provided application [`App`].
///
/// The [`initialize`](`Module::initialize`) method returns the modified application with the added
/// resource and system.
pub trait Module {
    /// Initialize the [`Module`] for the provided app.
    ///
    /// This method should be used to add the required resources and systems to the application.
    fn initialize(self, app: App) -> Result<App>;
}
