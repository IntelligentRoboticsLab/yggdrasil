use re_viewer::external::egui::{self, Frame, InnerResponse, RichText};

pub mod camera_calibration;
pub mod debug_systems;
pub mod resource;

pub const SIDE_PANEL_WIDTH: f32 = 400.0;
pub const PANEL_TOP_PADDING: f32 = 10.0;

const VIEW_SECTION_MARGIN: f32 = 5.0;
const HEADING_FONT_SIZE: f32 = 14.0;

pub fn view_section<R>(ui: &mut egui::Ui, heading: String, add_contents: impl FnOnce(&mut egui::Ui) -> R) -> InnerResponse<R> {
    let frame = Frame {
        outer_margin: egui::Margin::same(VIEW_SECTION_MARGIN),
        ..Default::default()
    };
    frame.show(ui, |ui| {
        ui.add_space(PANEL_TOP_PADDING);
        ui.label(RichText::new(heading).strong().size(HEADING_FONT_SIZE));
        ui.separator();
        add_contents(ui)
    })
}
