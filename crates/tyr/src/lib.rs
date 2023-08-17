pub use tyr_internal::*;
pub use tyr_macros::*;
pub use tyr_tasks::*;

/// `use tyr::prelude::*;` to import commonly used items.
pub mod prelude {
    pub use super::{
        system, wrap, App, IntoDependencySystem, Module, Res, ResMut, Resource, Storage,
    };
}
