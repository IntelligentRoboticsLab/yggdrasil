pub use tyr_internal::*;
pub use tyr_macros::*;

/// Tasks allow functions to run over multiple execution cycles.
pub mod tasks {
    pub use tyr_tasks::*;
}

/// `use tyr::prelude::*;` to import commonly used items.
pub mod prelude {
    pub use super::tasks::prelude::*;
    pub use super::{
        startup_system, system, App, Inspect, IntoDependencySystem, Module, Res, ResMut, Resource,
        Storage, SystemStage,
    };
}
