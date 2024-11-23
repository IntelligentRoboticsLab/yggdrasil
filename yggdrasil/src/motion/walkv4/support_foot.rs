use bevy::prelude::*;
use nidhogg::types::ForceSensitiveResistors;

use crate::prelude::Sensor;

use super::Side;

#[derive(Debug, Default, Clone, Resource)]
pub struct SupportFoot {
    side: Side,
}

pub(super) struct SupportFootPlugin;

impl Plugin for SupportFootPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SupportFoot>();
        app.add_systems(
            Sensor,
            update_support_foot.after(crate::sensor::fsr::force_sensitive_resistor_sensor),
        );
    }
}

fn update_support_foot(fsr: Res<ForceSensitiveResistors>, mut support_foot: ResMut<SupportFoot>) {
    let left_foot = fsr.left_foot.sum();
    let right_foot = fsr.right_foot.sum();

    let has_switched = match support_foot.side {
        Side::Left => right_foot,
        Side::Right => left_foot,
    } > 0.6;

    if has_switched {
        support_foot.side = support_foot.side.opposite();
    }
}
