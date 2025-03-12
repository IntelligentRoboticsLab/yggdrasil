use bevy::prelude::*;

pub struct BallTrackerPlugin;

impl Plugin for BallTrackerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, init_position_filter).
        add_systems(PreUpdate, update_);
    }
}
