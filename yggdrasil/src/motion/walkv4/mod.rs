use bevy::prelude::*;
use feet::FootPositions;

mod balancing;
mod feet;
mod gait;
mod hips;
mod mod_walk;
mod scheduling;
mod step;
mod support_foot;

pub struct Walkv4EnginePlugin;

impl Plugin for Walkv4EnginePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SwingFoot>();
        app.init_resource::<TargetFootPositions>();
        app.add_plugins((
            scheduling::MotionSchedulePlugin,
            gait::GaitPlugins,
            balancing::BalancingPlugin,
        ));
    }
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    #[default]
    Left,
    Right,
}

impl Side {
    #[must_use]
    pub fn opposite(self) -> Self {
        match self {
            Side::Left => Side::Right,
            Side::Right => Side::Left,
        }
    }
}

/// Resource containing the current swing foot of the walking engine.
#[derive(Resource, Clone, Copy, Debug, Default, Deref, DerefMut)]
pub struct SwingFoot(Side);

/// Resource containing the current requested [`FootPositions`].
#[derive(Debug, Default, Clone, Resource, Deref, DerefMut)]
pub struct TargetFootPositions(FootPositions);
