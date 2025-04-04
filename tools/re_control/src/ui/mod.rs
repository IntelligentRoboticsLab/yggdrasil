use rerun::external::egui::{
    self, scroll_area::ScrollAreaOutput, Frame, InnerResponse, RichText, ScrollArea,
};

use crate::connection::ConnectionState;

pub mod camera_calibration;
pub mod debug_systems;
pub mod fake_game_controller;
pub mod field_color;
pub mod resource;
pub mod selection_ui;
pub mod visual_referee;

pub const PANEL_TOP_PADDING: f32 = 10.0;

const VIEW_SECTION_MARGIN: i8 = 5;
const HEADING_FONT_SIZE: f32 = 14.0;

pub fn view_section<R>(
    ui: &mut egui::Ui,
    heading: String,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> InnerResponse<ScrollAreaOutput<R>> {
    let frame = Frame {
        outer_margin: egui::Margin::same(VIEW_SECTION_MARGIN),
        ..Default::default()
    };
    frame.show(ui, |ui| {
        ScrollArea::vertical()
            .auto_shrink([false, true])
            .show(ui, |ui| {
                ui.add_space(PANEL_TOP_PADDING);
                ui.label(RichText::new(heading).strong().size(HEADING_FONT_SIZE));
                ui.separator();
                add_contents(ui)
            })
    })
}

pub(crate) fn extra_title_bar_connection_ui(ui: &mut egui::Ui, connection: &ConnectionState) {
    let robot_connection_ip_addr = *connection.handle.addr().ip();
    let ip_addr_last_oct = robot_connection_ip_addr.octets()[3];

    // Find a possible corresponding name based on the last octet of the robot ip address
    let robot_name = if let Some(robot_config) = connection
        .possible_robot_connections
        .iter()
        .find(|config| config.number == ip_addr_last_oct)
    {
        format!("{} - ", robot_config.name)
    } else {
        "unknown - ".to_string()
    };

    // Show the ip associated with the socket of the `ControlViewer`
    ui.label(format!("{}{}", robot_name, robot_connection_ip_addr));
}
