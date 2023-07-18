use color_eyre::Result;
use nidhogg::{
    types::{Color, JointArray, LeftEye},
    NaoControlMessage,
};

use tyr::system;

use crate::filter::button::{ChestButton, HeadButtons};

#[derive(Default, PartialEq)]
pub enum WalkingEngineState {
    #[default]
    Standing,
    Walking,
}

#[derive(Default)]
pub struct WalkingEngine {
    pub state: WalkingEngineState,
}

#[system]
pub fn toggle_walking_engine(
    head_button: &HeadButtons,
    chest_button: &ChestButton,
    walking_engine: &mut WalkingEngine,
) -> Result<()> {
    if chest_button.is_pressed() && walking_engine.state == WalkingEngineState::Standing {
        walking_engine.state = WalkingEngineState::Walking;
    }

    if head_button.front.is_pressed() && walking_engine.state == WalkingEngineState::Walking {
        walking_engine.state = WalkingEngineState::Standing;
    }

    Ok(())
}

#[system]
pub fn walking_engine(
    walking_engine: &mut WalkingEngine,
    control_message: &mut NaoControlMessage,
) -> Result<()> {
    let color = match walking_engine.state {
        WalkingEngineState::Standing => Color::new(1.0, 0.0, 0.0),
        WalkingEngineState::Walking => Color::new(0.0, 1.0, 0.0),
    };

    control_message.left_eye = LeftEye::builder()
        .color_0_deg(color)
        .color_45_deg(color)
        .color_90_deg(color)
        .color_135_deg(color)
        .color_180_deg(color)
        .color_225_deg(color)
        .color_270_deg(color)
        .color_315_deg(color)
        .build();

    let stiffness = match walking_engine.state {
        WalkingEngineState::Standing => 1.0,
        WalkingEngineState::Walking => 0.6,
    };

    control_message.stiffness = JointArray::<f32>::builder()
        .left_hip_pitch(stiffness)
        .right_hip_pitch(stiffness)
        .left_hip_roll(stiffness)
        .right_hip_roll(stiffness)
        .left_ankle_pitch(stiffness)
        .right_ankle_pitch(stiffness)
        .left_knee_pitch(stiffness)
        .right_knee_pitch(stiffness)
        .left_hip_yaw_pitch(stiffness)
        .build();

    if walking_engine.state == WalkingEngineState::Standing {
        control_message.position = JointArray::<f32>::default();
    } else {
        control_message.position = JointArray::<f32>::builder()
            .left_ankle_pitch(-std::f32::consts::FRAC_PI_8)
            .right_ankle_pitch(-std::f32::consts::FRAC_PI_8)
            .left_knee_pitch(std::f32::consts::FRAC_PI_4)
            .right_knee_pitch(std::f32::consts::FRAC_PI_4)
            .left_hip_pitch(-std::f32::consts::FRAC_PI_6)
            .right_hip_pitch(-std::f32::consts::FRAC_PI_6)
            .build();
    }

    Ok(())
}
