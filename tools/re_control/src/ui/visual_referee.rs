use std::sync::{Arc, RwLock};

use re_control_comms::{protocol::ViewerMessage, viewer::ControlViewerHandle};
use rerun::external::egui;

use crate::re_control_view::ControlViewerData;

use super::view_section;

pub fn visual_referee_ui(
    ui: &mut egui::Ui,
    _viewer_data: Arc<RwLock<ControlViewerData>>,
    handle: &ControlViewerHandle,
) {
    view_section(ui, "Visual Referee".to_string(), |ui| {
        if ui.button("Recognize pose").clicked() {
            if let Err(error) = handle.send(ViewerMessage::VisualRefereeRecognition) {
                tracing::error!(?error, "Failed to send message");
            }
        }
    });
}
