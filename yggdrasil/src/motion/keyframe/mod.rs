pub mod executor;
pub mod manager;
pub mod types;
mod util;

use crate::prelude::*;
use bevy::prelude::*;

pub use manager::*;
pub use types::*;
pub use util::*;

/// Plugin that provides keyframe motion functionalities.
pub(super) struct KeyframePlugin;

impl Plugin for KeyframePlugin {
    fn build(&self, app: &mut App) {
        let mut animation_manager: AnimationManager = AnimationManager::new();
        // Add new motions here!
        animation_manager
            .add_motion(
                MotionType::StandupBack,
                "./assets/motions/standup_back.toml",
            )
            .expect("failed to add standup motion (back)");
        animation_manager
            .add_motion(
                MotionType::StandupStomach,
                "./assets/motions/standup_stomach.toml",
            )
            .expect("failed to add standup motion (stomach)");

        app.insert_resource(animation_manager);
        app.add_systems(PostWrite, executor::animation_executor);
    }
}
