pub use tyr_internal::*;
pub use tyr_macros::*;

/// Tasks allow functions to run over multiple execution cycles.
///
/// To create a task returning `T`, simply add the [`Task<T>`](`tasks::Task`) to the [`App`] as a [`Resource`].
///
/// For I/O heavy tasks such as reading from files or networks, use the [`AsyncDispatcher`](`tasks::AsyncDispatcher`).
///
/// For compute heavy tasks such as large calculations, use the [`ComputeDispatcher`](`tasks::ComputeDispatcher`).
pub mod tasks {
    pub use tyr_tasks::*;
}

/// `use tyr::prelude::*;` to import commonly used items.
pub mod prelude {
    pub use super::{system, App, IntoDependencySystem, Module, Res, ResMut, Resource, Storage};
}
