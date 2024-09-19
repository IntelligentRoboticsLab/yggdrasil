use crate::Tag;
use bevy::prelude::*;

pub fn task_finished<T: Send + Sync + 'static>(query: Query<&Tag<T>>) -> bool {
    query.is_empty()
}
