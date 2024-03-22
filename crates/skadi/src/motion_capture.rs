use std::time::Duration;

use nidhogg::{types::{FillExt, JointArray}, NaoControlMessage, NaoState};
use yggdrasil::filter::button::{LeftFootButtons, RightFootButtons, HeadButtons};
use serde::{Deserialize, Serialize};
use serde_json;
use std::fs::File;
use std::collections::HashMap;
use std::path::Path;
use std::string::ToString;
use miette::{IntoDiagnostic, Result};
use yggdrasil::prelude::*;

// I have setup a connection to Lola, however it it necessary to write a message to the backend very 12ms. 
// The write call to the backend end is blocking. So if you make a loop and write in it every iteration, that should work.
// The difficult thing wil be, reading user input, but not blocking on it and still making sure messages are sent. 

// Dario note: We will need to add threads, async, etc. USE TOKIO, WATCH TUTORIALS


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

#[derive(Debug, Serialize, Deserialize, Clone)]
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum FailRoutine {
    Retry,
    Abort,
    Catch,
}

impl Module for Sk {
    fn initialize(self, app: App) -> Result<App> {
        Ok(app
            .add_resource(Resource::new(MotionCapResources::new()))?
            .add_system(register_button_press))
    }
}

pub struct MotionCapResources {
    pub locked: bool,
    pub new_motion_init: bool,
    pub motion_counter: u32,
}

impl MotionCapResources {
    fn new() -> MotionCapResources {
        MotionCapResources { locked: false, new_motion_init: false, motion_counter: 0}
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
            println!("Initialize new motion");
            let submotion_path = Path::new("/home/nao/assets/motions/submotions")
                    .join(format!("new_motion{}", motioncapresources.motion_counter.clone()))
                    .with_extension("json");

            serde_json::to_writer_pretty(&File::create(submotion_path).into_diagnostic()?, &naostate.position.clone()).into_diagnostic()?;
            //TODO: initialize a new motion, use svisser/damageprevention (motiontypes.rs en de assets). create json file in correct format in order for us to add keyframes
        }
    }
    if head_button.rear.is_tapped() {
        println!("Head Rear Pressed!");
        println!("{:?}", naostate.position.clone());
        
    }
    Ok(())
}
