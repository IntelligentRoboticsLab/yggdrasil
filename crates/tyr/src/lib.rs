pub use tyr_internal::*;
pub use tyr_macros::*;

/// Tasks allow functions to run over multiple execution cycles.
pub mod tasks {
    pub use tyr_tasks::*;
}

/// `use tyr::prelude::*;` to import commonly used items.
pub mod prelude {
    pub use super::{system, App, IntoDependencySystem, Module, Res, ResMut, Resource, Storage};
}
