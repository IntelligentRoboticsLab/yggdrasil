pub use tyr_internal::*;
pub use tyr_macros::*;

/// Tasks allow functions to run over multiple execution cycles.
///
/// # Example
///
/// ```rust
/// use std::time::Duration;
/// use miette::Result;
/// use tokio::time::sleep;
/// use tyr::{
///     prelude::*,
///     tasks::{AsyncDispatcher, Task},
/// };
///
/// #[derive(Default)]
/// struct Counter(u64);
/// struct Name(String);
///
/// // this is a function that needs to wait for a while, blocking our main thread
/// async fn receive_name(duration: Duration) -> Name {
///     sleep(duration).await;
///     Name("Daphne".to_string())
/// }
///
/// #[system]
/// fn dispatch_name(ad: &AsyncDispatcher, task: &mut Task<Name>) -> Result<()> {
///     // We dispatch a future onto a separate thread, and marks the task as alive.
///     //
///     // If the task is already alive, nothing is dispatched and the function fails.
///     //
///     // In this case, we want to do nothing while a task is alive, so we ignore
///     // the result and just return from the function.
///     let _ = ad.try_dispatch(&mut task, receive_name(Duration::from_secs(1)));
///
///     Ok(())
/// }
///
/// #[system]
/// fn poll_name(task: &mut Task<Name>, counter: &mut Counter) -> Result<()> {
///     // If the task hasn't completed yet, we return early
///     let Some(name) = task.poll() else {
///         return Ok(());
///     };
///
///     println!("Hello, {}! Counter is at {}", name.0, counter.0);
///     counter.0 = 0;
///
///     Ok(())
/// }
///
/// #[system]
/// fn time_critical_task(counter: &mut Counter) -> Result<()> {
///     // This will still run many times a second even though
///     // `receive_name` is waiting for 1 second
///     counter.0 += 1;
///
///     Ok(())
/// }
/// ```
pub mod tasks {
    pub use tyr_tasks::*;
}

/// `use tyr::prelude::*;` to import commonly used items.
pub mod prelude {
    pub use super::{system, App, IntoDependencySystem, Module, Res, ResMut, Resource, Storage};
}
