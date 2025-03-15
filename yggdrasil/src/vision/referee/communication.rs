use bevy::prelude::*;

use crate::communication::{TeamCommunication, TeamMessage};

use super::{recognize::RefereePoseRecognized, RefereePose};
pub struct RefereePoseCommunicationPlugin;

impl Plugin for RefereePoseCommunicationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (send_message, receive_message))
            .add_event::<ReceivedRefereePose>();
    }
}

fn send_message(
    mut recognized_pose: EventReader<RefereePoseRecognized>,
    mut tc: ResMut<TeamCommunication>,
) {
    for pose_event in recognized_pose.read() {
        if pose_event.pose == RefereePose::Ready {
            tc.outbound_mut()
                .update_or_push(TeamMessage::RecognizedRefereePose(pose_event.pose))
                .expect("unable to encode recognized referee pose");
        }
    }
}

fn receive_message(
    mut tc: ResMut<TeamCommunication>,
    mut writer: EventWriter<ReceivedRefereePose>,
) {
    let incoming_msg = tc.inbound_mut().take_map(|_, _, msg| match msg {
        TeamMessage::RecognizedRefereePose(pose) => Some(*pose),
        _ => None,
    });

    if let Some((_, _, pose)) = incoming_msg {
        writer.send(ReceivedRefereePose { pose });
    }
}

#[derive(Event)]
pub struct ReceivedRefereePose {
    pub pose: RefereePose,
}
