use crate::App;
use color_eyre::Result;

/// A module represents a collection of resources and systems that can be added to an application [`App`].
///
/// Modules encapsulate related functionality and provide a way to organize and modularize the codebase.
///
/// They define an [`Module::initialize`] method, which is responsible for adding the required resources and systems
/// to the provided [`App`].
///
/// # Example
///
/// ```
/// use tyr_internal::{App, Resource, Module};
/// use color_eyre::Result;
///
/// // Define a module struct
/// struct FooModule;
///
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
/// ```
///
/// In this example, the `FooModule` adds a resource [`Resource::new(42_i32)`] and a system [`foo_system`]
/// to the provided application [`App`].
///
/// The [`Module::initialize`] method returns the modified application with the added
/// resource and system.
pub trait Module {
    /// Initialize the [`Module`] for the provided [`App`].
    ///
    /// This method should be used to add the required resources and systems to the [`App`].
    fn initialize(self, app: App) -> Result<App>;
}
