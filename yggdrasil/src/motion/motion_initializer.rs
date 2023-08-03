/// Initializes the `MotionManager`. Adds motions to the `MotionManger` by reading
/// and deserializing the motions from motion files. Then adds the `MotionManager`
/// as resource
///
/// # Arguments
///
/// * `storage` - System storage.
pub fn motion_manager_initializer(storage: &mut Storage) -> Result<()> {
    let mut motion_manager = MotionManager::new();
    motion_manager.add_motion(
        MotionType::SitDownFromStand,
        "./sit_down_from_stand_motion.json",
    )?;
    motion_manager.add_motion(
        MotionType::StandUpFromSit,
        "./stand_up_from_sit_motion.json",
    )?;

    // TODO: remove this, this is for testing
    motion_manager.start_new_motion(MotionType::StandUpFromSit);
    storage.add_resource(Resource::new(motion_manager))?;

    Ok(())
}
