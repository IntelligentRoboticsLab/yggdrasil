// TODO: documentation

mod asynchronous;
mod compute;
mod task;

pub mod tasks {
    pub mod asynchronous {
        pub use crate::asynchronous::*;
    }

    pub mod compute {
        pub use crate::compute::*;
    }

    pub use super::task::*;
}
