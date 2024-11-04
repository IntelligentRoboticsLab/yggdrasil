use std::collections::HashMap;

use re_viewer::external::egui::{Frame, Margin, Rounding};
use rerun::external::ecolor::Color32;

pub const LAST_UPDATE_COLOR: Color32 = Color32::from_gray(100);

pub struct FrameStyleMap(HashMap<String, Frame>);

impl FrameStyleMap {
    pub fn get_or_default(&self, frame_style_name: String) -> Frame {
        if let Some(style) = self.0.get(&frame_style_name) {
            *style
        } else {
            Frame::default()
        }
    }
}

impl Default for FrameStyleMap {
    fn default() -> Self {
        let mut style_map = HashMap::new();

        style_map.insert(
            "refresh_button".to_string(),
            Frame {
                fill: Color32::BLACK, // Background color of the frame (around the button)
                rounding: Rounding::same(5.0), // Rounding for the frame
                inner_margin: Margin::same(3.0), // Padding inside the frame
                ..Default::default()
            },
        );

        style_map.insert(
            "override_button".to_string(),
            Frame {
                fill: Color32::BLACK, // Background color of the frame (around the button)
                rounding: Rounding::same(5.0), // Rounding for the frame
                inner_margin: Margin::same(3.0), // Padding inside the frame
                ..Default::default()
            },
        );

        FrameStyleMap(style_map)
    }
}
