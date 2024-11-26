use bevy::prelude::Event;
use uuid::Uuid;

#[derive(Event)]
pub struct ViewerConnected(pub Uuid);

#[derive(Event)]
pub struct DebugEnabledSystemUpdated;
