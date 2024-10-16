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
        let mut keyframe_executor = KeyframeExecutor::new();
        // Add new motions here!
        keyframe_executor
            .add_motion(
                MotionType::StandupBack,
                "./assets/motions/standup_back.toml",
            )
            .expect("failed to add standup motion (back)");
        keyframe_executor
            .add_motion(
                MotionType::StandupStomach,
                "./assets/motions/standup_stomach.toml",
            )
            .expect("failed to add standup motion (stomach)");

        app.insert_resource(keyframe_executor);
        app.add_systems(PostWrite, executor::keyframe_executor);
    }
}
