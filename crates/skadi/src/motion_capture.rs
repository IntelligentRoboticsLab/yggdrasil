use std::{path::PathBuf, time::Duration};

use nidhogg::{types::{FillExt, JointArray}, NaoControlMessage, NaoState};
use yggdrasil::filter::button::{LeftFootButtons, RightFootButtons, HeadButtons};
use serde::{Deserialize, Serialize};
use serde_json;
use std::fs::File;
use std::path::Path;
use miette::{IntoDiagnostic, Result};
use yggdrasil::prelude::*;



pub struct Sk;

// Below some structs that are compatible with Stephan's motion manager
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Movement {
    /// Movement target joint positions.
    pub target_position: JointArray<f32>,
    /// Movement duration.
    pub duration: Duration,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ConditionalVariable {
    GyroscopeX,
    GyroscopeY,
    AngleX,
    AngleY,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct SubMotion {
    pub joint_stifness: f32,
    pub chest_angle_bound_upper: f32,
    pub chest_angle_bound_lower: f32,
    pub fail_routine: FailRoutine,
    pub conditions: Vec<MotionCondition>,
    pub keyframes: Vec<Movement>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MotionCondition {
    pub variable: ConditionalVariable,
    pub min: f32,
    pub max: f32,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub enum FailRoutine {
    #[default]
    Retry,
    Abort,
    Catch,
}

impl Module for Sk {
    fn initialize(self, app: App) -> Result<App> {
        Ok(app
            .add_resource(Resource::new(MotionCapResources::new()))?
            .add_system(register_button_press.after(yggdrasil::filter::button::button_filter)))
    }
}

pub struct MotionCapResources {
    pub locked: bool,
    pub new_motion_init: bool,
    pub motion_counter: u32,
    pub currentmotion: SubMotion,
    pub submotion_path: PathBuf,
}

impl MotionCapResources {
    fn new() -> MotionCapResources {
        MotionCapResources { locked: false, new_motion_init: false, motion_counter: 0, currentmotion: SubMotion::default(), submotion_path: PathBuf::default()}
    }
}

#[system]
fn register_button_press(
    head_button: &mut HeadButtons,
    left_button: &mut LeftFootButtons,
    right_button: &mut RightFootButtons,
    motioncapresources: &mut MotionCapResources,
    nao_control_message: &mut NaoControlMessage,
    naostate: &mut NaoState,
) -> Result<()> {
    if left_button.left.is_tapped() {
        println!("Left Pressed!");
    }
    if right_button.left.is_tapped() {
        println!("Right Pressed!");
    }
    if head_button.front.is_tapped() {
        if !motioncapresources.locked {
            println!("Joints locked!");
            nao_control_message.position = naostate.position.clone();
            nao_control_message.stiffness = JointArray::<f32>::fill(0.3);
            motioncapresources.locked = true;
        }
        else {
            println!("Joints unlocked!");
            nao_control_message.stiffness =  JointArray::<f32>::fill(0.0);
            motioncapresources.locked = false;
        }
    }
    if head_button.middle.is_tapped() {
        if !motioncapresources.new_motion_init {
            clearscreen::clear().expect("failed to clear screen");
            println!("New motion initialized.");
            motioncapresources.submotion_path = Path::new("/home/nao/assets/motions/submotions")
                    .join(format!("new_motion{}", motioncapresources.motion_counter.clone()))
                    .with_extension("json");
            
            motioncapresources.currentmotion = SubMotion {
                joint_stifness: 0.7,
                chest_angle_bound_upper: 0.4,
                chest_angle_bound_lower: -0.4,
                fail_routine: FailRoutine::Retry,
                conditions: Vec::new(),
                keyframes: Vec::new()
            };
            motioncapresources.new_motion_init = true;
        }
        else {
            println!("Saving file to {:?}", motioncapresources.submotion_path);
            serde_json::to_writer_pretty(&File::create(motioncapresources.submotion_path.clone()).into_diagnostic()?, &motioncapresources.currentmotion).into_diagnostic()?;
            motioncapresources.new_motion_init = false;
            motioncapresources.motion_counter += 1;
        }
    }
    if head_button.rear.is_tapped() {
        if motioncapresources.new_motion_init {
            println!("Keyframe recorded:");
            println!("{:?}", naostate.position.clone());
            motioncapresources.currentmotion.keyframes.push(Movement{target_position: naostate.position.clone(), duration: Duration::from_secs(1)});
        }
        else {
            println!("No motion recording active, press the middle headbutton to initialize a movement.")
        }        
        
    }
    Ok(())
}
