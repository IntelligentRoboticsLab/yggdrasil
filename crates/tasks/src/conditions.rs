use crate::Tag;
use bevy::prelude::*;

#[must_use]
#[allow(clippy::needless_pass_by_value)]
pub fn task_finished<T: Send + Sync + 'static>(query: Query<&Tag<T>>) -> bool {
    query.is_empty()
}
