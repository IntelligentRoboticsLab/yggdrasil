use crate::motion::motion_executer::reached_position;
use crate::motion::motion_manager::MotionManager;
use crate::motion::motion_types::{Motion, MotionType, Movement};

pub struct DamagePreventionModule;

impl Module for DamagePreventionModule {
    fn initialize(self, app: App) -> Result<App> {
        app.add_system(pose_filter)
            .add_resource(Resource::new(Pose::default()))
    }
}

#[system]
fn fallcatch() {
    // setting the active_motion variable to the final keyframe that the current motion is working towards
    match motion_type {
        Some(selected_motion) => {
            mmng.start_new_motion(selected_motion);
            match mmng.get_active_motion() {
                Some(active_motion) => damprevresources.active_motion = Some(active_motion.motion),
                _ => (),
            }

            damprevresources.brace_for_impact = false;
        }
        None => (),
    }
}
