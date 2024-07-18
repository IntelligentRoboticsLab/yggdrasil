use std::ops::{Index, IndexMut};
use std::{path::PathBuf, time::Duration};

use miette::{IntoDiagnostic, Result};
use nidhogg::types::{
    ArmJoints, HeadJoints, LeftArmJoints, LeftLegJoints, LegJoints, RightArmJoints, RightLegJoints,
};
use nidhogg::{
    types::{FillExt, JointArray},
    NaoState,
};
use serde::{Deserialize, Serialize};
use serde_json;
use std::fs::File;
use std::path::Path;
use yggdrasil::nao::manager::NaoManager;
use yggdrasil::nao::manager::Priority;
use yggdrasil::prelude::*;
use yggdrasil::sensor::button::{HeadButtons, LeftFootButtons, RightFootButtons};

pub struct SkadiModule;

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
    pub exitwaittime: f32,
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

impl Module for SkadiModule {
    fn initialize(self, app: App) -> Result<App> {
        Ok(app
            .init_resource::<MotionCapResources>()?
            .add_system(register_button_press.after(yggdrasil::sensor::button::button_filter)))
    }
}

/* Resource saving the following data:
    - locked: the locked/unlocked status of every joint
    - new_motion_init: if a motion capture is active, of if we can initialize a new motion
    - motion_counter: the n'th motion that we are on (for labelling them correctly, and building a nice path to save them)
    - currentmotion: the current motion that we are filling with keyframes, opened with motion init (middle head button) and closed and saved once motion init button is pressed again
    - submotion_path: the path to our current motion we are capturing
    - selected_group: the joint group we currently are focused on by pressing the feet buttons to rotate through them, using this to lock/unlock specific joints
    - joint_groups: if the joint(s) and/or jointgroups are currently locked or unlocked
*/

#[derive(Default)]
pub struct MotionCapResources {
    pub locked: bool,
    pub new_motion_init: bool,
    pub motion_counter: u32,
    pub currentmotion: SubMotion,
    pub submotion_path: PathBuf,
    pub selected_group: usize,
    pub joint_groups: JointGroups,
}

#[derive(Default, Debug)]
pub struct JointGroups {
    fullbody: bool,
    botharms: bool,
    bothlegs: bool,
    leftarm: bool,
    rightarm: bool,
    leftleg: bool,
    rightleg: bool,
    head: bool,
}

impl JointGroups {
    pub fn all_true(&self) -> bool {
        self.fullbody
            && self.botharms
            && self.bothlegs
            && self.leftarm
            && self.rightarm
            && self.leftleg
            && self.rightleg
            && self.head
    }
}

impl Index<usize> for JointGroups {
    type Output = bool;

    fn index(&self, index: usize) -> &Self::Output {
        match index {
            0 => &self.fullbody,
            1 => &self.botharms,
            2 => &self.bothlegs,
            3 => &self.leftarm,
            4 => &self.rightarm,
            5 => &self.leftleg,
            6 => &self.rightleg,
            7 => &self.head,
            _ => panic!("Index out of range"),
        }
    }
}

impl IndexMut<usize> for JointGroups {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        match index {
            0 => &mut self.fullbody,
            1 => &mut self.botharms,
            2 => &mut self.bothlegs,
            3 => &mut self.leftarm,
            4 => &mut self.rightarm,
            5 => &mut self.leftleg,
            6 => &mut self.rightleg,
            7 => &mut self.head,
            _ => panic!("Index out of range"),
        }
    }
}

impl MotionCapResources {
    pub fn printname(&self) {
        if self.selected_group == 0 {
            println!("Full body joints selected.")
        } else if self.selected_group == 1 {
            println!("Both arms selected.")
        } else if self.selected_group == 2 {
            println!("Both legs selected.")
        } else if self.selected_group == 3 {
            println!("Left arm selected.")
        } else if self.selected_group == 4 {
            println!("Right arm selected.")
        } else if self.selected_group == 5 {
            println!("Left leg selected.")
        } else if self.selected_group == 6 {
            println!("Right leg selected.")
        } else if self.selected_group == 7 {
            println!("Head selected.")
        }
    }
}

/* How to use: Skadi
    Initialize motion: To initialize a motion, we can press the middle head button to create the first motion. WARNING! This action does not create the first key-
    frame, you will have to do this step manually.
    Locking robot in position: To lock the robot in its position, or lock specific joints in position for easier 'moulding', press the front head button. To unlock
    the joint(s), press the front head button again.
    Rotating joint selection: To scroll through the selected joints, use the feet buttons to scroll forwards with the right foot, and backwards with the left foot.
    Add keyframe: You can add a keyframe to the motion by pressing the rear head button, which will save the joint positions. This is only possible if a motion is
    initialized, this will also be shown in the terminal.
*/
#[system]
fn register_button_press(
    head_button: &mut HeadButtons,
    left_button: &mut LeftFootButtons,
    right_button: &mut RightFootButtons,
    motion_cap_resources: &mut MotionCapResources,
    nao_manager: &mut NaoManager,
    naostate: &mut NaoState,
) -> Result<()> {
    if left_button.left.is_tapped() {
        motion_cap_resources.selected_group += 1;
        motion_cap_resources.selected_group %= 8;
        motion_cap_resources.printname();
    }
    if right_button.left.is_tapped() {
        if motion_cap_resources.selected_group == 0 {
            motion_cap_resources.selected_group = 7;
            motion_cap_resources.printname();
        } else {
            motion_cap_resources.selected_group -= 1;
            motion_cap_resources.printname();
        }
    }
    if head_button.front.is_tapped() {
        if motion_cap_resources.selected_group == 0 {
            if !motion_cap_resources.joint_groups[motion_cap_resources.selected_group] {
                println!("Full body joints locked");
                let head_stiffness = HeadJoints::fill(0.5);
                let arm_stiffness = ArmJoints::fill(0.5);
                let leg_stiffness = LegJoints::fill(0.5);
                nao_manager.set_all(
                    naostate.position.clone(),
                    head_stiffness,
                    arm_stiffness,
                    leg_stiffness,
                    Priority::Critical,
                );
                println!("{:?}", naostate.stiffness.clone());
                motion_cap_resources.joint_groups[motion_cap_resources.selected_group] = true;
                motion_cap_resources.joint_groups.botharms = true;
                motion_cap_resources.joint_groups.bothlegs = true;
                motion_cap_resources.joint_groups.rightarm = true;
                motion_cap_resources.joint_groups.leftarm = true;
                motion_cap_resources.joint_groups.rightleg = true;
                motion_cap_resources.joint_groups.leftleg = true;
                motion_cap_resources.joint_groups.head = true;
                println!("{:?}", naostate.stiffness.clone())
            } else {
                println!("Full body joints unlocked");
                let head_stiffness = HeadJoints::fill(0.0);
                let arm_stiffness = ArmJoints::fill(0.0);
                let leg_stiffness = LegJoints::fill(0.0);
                nao_manager.set_all(
                    naostate.position.clone(),
                    head_stiffness,
                    arm_stiffness,
                    leg_stiffness,
                    Priority::Critical,
                );
                motion_cap_resources.joint_groups[motion_cap_resources.selected_group] = false;
                motion_cap_resources.joint_groups.botharms = false;
                motion_cap_resources.joint_groups.bothlegs = false;
                motion_cap_resources.joint_groups.rightarm = false;
                motion_cap_resources.joint_groups.leftarm = false;
                motion_cap_resources.joint_groups.rightleg = false;
                motion_cap_resources.joint_groups.leftleg = false;
                motion_cap_resources.joint_groups.head = false;
                println!("{:?}", naostate.stiffness.clone())
            }
        } else if motion_cap_resources.selected_group == 1 {
            if !motion_cap_resources.joint_groups[motion_cap_resources.selected_group] {
                println!("Both arm joints locked");

                let left_arm_stiffness = LeftArmJoints::fill(0.5);
                let right_arm_stiffness = RightArmJoints::fill(0.5);

                let arms_set = ArmJoints {
                    left_arm: left_arm_stiffness,
                    right_arm: right_arm_stiffness,
                };

                nao_manager.set_arms(
                    naostate.position.arm_joints().clone(),
                    arms_set,
                    Priority::Critical,
                );

                motion_cap_resources.joint_groups[motion_cap_resources.selected_group] = true;
                motion_cap_resources.joint_groups.rightarm = true;
                motion_cap_resources.joint_groups.leftarm = true;
            } else {
                println!("Both arm joints unlocked");
                let left_arm_stiffness = LeftArmJoints::fill(0.0);
                let right_arm_stiffness = RightArmJoints::fill(0.0);

                let arms_set = ArmJoints {
                    left_arm: left_arm_stiffness,
                    right_arm: right_arm_stiffness,
                };

                nao_manager.set_arms(
                    naostate.position.arm_joints().clone(),
                    arms_set,
                    Priority::Critical,
                );

                motion_cap_resources.joint_groups[motion_cap_resources.selected_group] = false;
                motion_cap_resources.joint_groups.rightarm = false;
                motion_cap_resources.joint_groups.leftarm = false;
            }
        } else if motion_cap_resources.selected_group == 2 {
            if !motion_cap_resources.joint_groups[motion_cap_resources.selected_group] {
                println!("Both leg joints locked");

                let legs_set = LegJoints::fill(0.5);

                nao_manager.set_legs(
                    naostate.position.leg_joints().clone(),
                    legs_set,
                    Priority::Critical,
                );

                motion_cap_resources.joint_groups[motion_cap_resources.selected_group] = true;
                motion_cap_resources.joint_groups.rightleg = true;
                motion_cap_resources.joint_groups.leftleg = true;
            } else {
                println!("Both leg joints unlocked");
                let legs_set = LegJoints::fill(0.0);

                nao_manager.set_legs(
                    naostate.position.leg_joints().clone(),
                    legs_set,
                    Priority::Critical,
                );

                motion_cap_resources.joint_groups[motion_cap_resources.selected_group] = false;
                motion_cap_resources.joint_groups.rightleg = false;
                motion_cap_resources.joint_groups.leftleg = false;
            }
        } else if motion_cap_resources.selected_group == 3 {
            if !motion_cap_resources.joint_groups[motion_cap_resources.selected_group] {
                println!("Left arm joints locked");

                if motion_cap_resources.joint_groups.rightarm {
                    let left_arm = LeftArmJoints::fill(0.5);
                    let right_arm = RightArmJoints::fill(0.5);

                    let arms_set = ArmJoints {
                        left_arm,
                        right_arm,
                    };

                    nao_manager.set_arms(
                        naostate.position.arm_joints().clone(),
                        arms_set,
                        Priority::Critical,
                    );
                } else if !motion_cap_resources.joint_groups.rightarm {
                    let left_arm = LeftArmJoints::fill(0.5);
                    let right_arm = RightArmJoints::fill(0.0);

                    let arms_set = ArmJoints {
                        left_arm,
                        right_arm,
                    };

                    nao_manager.set_arms(
                        naostate.position.arm_joints().clone(),
                        arms_set,
                        Priority::Critical,
                    );
                }

                motion_cap_resources.joint_groups[motion_cap_resources.selected_group] = true;
            } else {
                println!("Left arm joints unlocked");
                if motion_cap_resources.joint_groups.rightarm {
                    let left_arm = LeftArmJoints::fill(0.0);
                    let right_arm = RightArmJoints::fill(0.5);

                    let arms_set = ArmJoints {
                        left_arm,
                        right_arm,
                    };

                    nao_manager.set_arms(
                        naostate.position.arm_joints().clone(),
                        arms_set,
                        Priority::Critical,
                    );
                } else if !motion_cap_resources.joint_groups.rightarm {
                    let left_arm = LeftArmJoints::fill(0.0);
                    let right_arm = RightArmJoints::fill(0.0);

                    let arms_set = ArmJoints {
                        left_arm,
                        right_arm,
                    };

                    nao_manager.set_arms(
                        naostate.position.arm_joints().clone(),
                        arms_set,
                        Priority::Critical,
                    );
                }

                motion_cap_resources.joint_groups[motion_cap_resources.selected_group] = false;
            }
        } else if motion_cap_resources.selected_group == 4 {
            if !motion_cap_resources.joint_groups[motion_cap_resources.selected_group] {
                println!("Right arm joints locked");
                if motion_cap_resources.joint_groups.leftarm {
                    let left_arm = LeftArmJoints::fill(0.5);
                    let right_arm = RightArmJoints::fill(0.5);

                    let arms_set = ArmJoints {
                        left_arm,
                        right_arm,
                    };

                    nao_manager.set_arms(
                        naostate.position.arm_joints().clone(),
                        arms_set,
                        Priority::Critical,
                    );
                } else if !motion_cap_resources.joint_groups.leftarm {
                    let left_arm = LeftArmJoints::fill(0.0);
                    let right_arm = RightArmJoints::fill(0.5);

                    let arms_set = ArmJoints {
                        left_arm,
                        right_arm,
                    };

                    nao_manager.set_arms(
                        naostate.position.arm_joints().clone(),
                        arms_set,
                        Priority::Critical,
                    );
                }
                motion_cap_resources.joint_groups[motion_cap_resources.selected_group] = true;
            } else {
                println!("Right arm joints unlocked");
                if motion_cap_resources.joint_groups.leftarm {
                    let left_arm = LeftArmJoints::fill(0.5);
                    let right_arm = RightArmJoints::fill(0.0);

                    let arms_set = ArmJoints {
                        left_arm,
                        right_arm,
                    };

                    nao_manager.set_arms(
                        naostate.position.arm_joints().clone(),
                        arms_set,
                        Priority::Critical,
                    );
                } else if !motion_cap_resources.joint_groups.rightarm {
                    let left_arm = LeftArmJoints::fill(0.0);
                    let right_arm = RightArmJoints::fill(0.0);

                    let arms_set = ArmJoints {
                        left_arm,
                        right_arm,
                    };

                    nao_manager.set_arms(
                        naostate.position.arm_joints().clone(),
                        arms_set,
                        Priority::Critical,
                    );
                }

                motion_cap_resources.joint_groups[motion_cap_resources.selected_group] = false;
            }
        } else if motion_cap_resources.selected_group == 5 {
            if !motion_cap_resources.joint_groups[motion_cap_resources.selected_group] {
                println!("Left leg joints locked");
                if motion_cap_resources.joint_groups.rightleg {
                    let left_leg = LeftLegJoints::fill(0.5);
                    let right_leg = RightLegJoints::fill(0.5);

                    let legs_set = LegJoints {
                        left_leg,
                        right_leg,
                    };

                    nao_manager.set_legs(
                        naostate.position.leg_joints().clone(),
                        legs_set,
                        Priority::Critical,
                    );
                } else if !motion_cap_resources.joint_groups.rightleg {
                    let left_leg = LeftLegJoints::fill(0.5);
                    let right_leg = RightLegJoints::fill(0.0);

                    let legs_set = LegJoints {
                        left_leg,
                        right_leg,
                    };

                    nao_manager.set_legs(
                        naostate.position.leg_joints().clone(),
                        legs_set,
                        Priority::Critical,
                    );
                }

                motion_cap_resources.joint_groups[motion_cap_resources.selected_group] = true;
            } else {
                println!("Left leg joints unlocked");
                if motion_cap_resources.joint_groups.rightleg {
                    let left_leg = LeftLegJoints::fill(0.0);
                    let right_leg = RightLegJoints::fill(0.5);

                    let legs_set = LegJoints {
                        left_leg,
                        right_leg,
                    };

                    nao_manager.set_legs(
                        naostate.position.leg_joints().clone(),
                        legs_set,
                        Priority::Critical,
                    );
                } else if !motion_cap_resources.joint_groups.rightleg {
                    let left_leg = LeftLegJoints::fill(0.0);
                    let right_leg = RightLegJoints::fill(0.0);

                    let legs_set = LegJoints {
                        left_leg,
                        right_leg,
                    };

                    nao_manager.set_legs(
                        naostate.position.leg_joints().clone(),
                        legs_set,
                        Priority::Critical,
                    );
                }

                motion_cap_resources.joint_groups[motion_cap_resources.selected_group] = false;
            }
        } else if motion_cap_resources.selected_group == 6 {
            if !motion_cap_resources.joint_groups[motion_cap_resources.selected_group] {
                println!("Right leg joints locked");
                if motion_cap_resources.joint_groups.leftleg {
                    let left_leg = LeftLegJoints::fill(0.5);
                    let right_leg = RightLegJoints::fill(0.5);

                    let legs_set = LegJoints {
                        left_leg,
                        right_leg,
                    };

                    nao_manager.set_legs(
                        naostate.position.leg_joints().clone(),
                        legs_set,
                        Priority::Critical,
                    );
                } else if !motion_cap_resources.joint_groups.leftleg {
                    let left_leg = LeftLegJoints::fill(0.0);
                    let right_leg = RightLegJoints::fill(0.5);

                    let legs_set = LegJoints {
                        left_leg,
                        right_leg,
                    };

                    nao_manager.set_legs(
                        naostate.position.leg_joints().clone(),
                        legs_set,
                        Priority::Critical,
                    );
                }

                motion_cap_resources.joint_groups[motion_cap_resources.selected_group] = true;
            } else {
                println!("Right leg joints unlocked");
                if motion_cap_resources.joint_groups.leftleg {
                    let left_leg = LeftLegJoints::fill(0.5);
                    let right_leg = RightLegJoints::fill(0.0);

                    let legs_set = LegJoints {
                        left_leg,
                        right_leg,
                    };

                    nao_manager.set_legs(
                        naostate.position.leg_joints().clone(),
                        legs_set,
                        Priority::Critical,
                    );
                } else if !motion_cap_resources.joint_groups.leftleg {
                    let left_leg = LeftLegJoints::fill(0.0);
                    let right_leg = RightLegJoints::fill(0.0);

                    let legs_set = LegJoints {
                        left_leg,
                        right_leg,
                    };

                    nao_manager.set_legs(
                        naostate.position.leg_joints().clone(),
                        legs_set,
                        Priority::Critical,
                    );
                }

                motion_cap_resources.joint_groups[motion_cap_resources.selected_group] = false;
            }
        } else if motion_cap_resources.selected_group == 7 {
            if !motion_cap_resources.joint_groups[motion_cap_resources.selected_group] {
                println!("Head joints locked");
                let head = HeadJoints::fill(0.5);

                nao_manager.set_head(
                    naostate.position.head_joints().clone(),
                    head,
                    Priority::Critical,
                );

                motion_cap_resources.joint_groups[motion_cap_resources.selected_group] = true;
            } else {
                println!("Head joints unlocked");
                let head = HeadJoints::fill(0.0);

                nao_manager.set_head(
                    naostate.position.head_joints().clone(),
                    head,
                    Priority::Critical,
                );

                motion_cap_resources.joint_groups[motion_cap_resources.selected_group] = false;
            }
        }
    }
    if head_button.middle.is_tapped() {
        if !motion_cap_resources.new_motion_init {
            clearscreen::clear().expect("failed to clear screen");
            println!("New motion initialized.");
            motion_cap_resources.submotion_path = Path::new("/home/nao/assets/motions")
                .join(format!(
                    "new_motion{}",
                    motion_cap_resources.motion_counter.clone()
                ))
                .with_extension("json");

            motion_cap_resources.currentmotion = SubMotion {
                joint_stifness: 0.7,
                chest_angle_bound_upper: 0.4,
                chest_angle_bound_lower: -0.4,
                fail_routine: FailRoutine::Retry,
                exitwaittime: 0.0,
                conditions: Vec::new(),
                keyframes: Vec::new(),
            };
            motion_cap_resources.new_motion_init = true;
        } else {
            println!("Saving file to {:?}", motion_cap_resources.submotion_path);
            serde_json::to_writer_pretty(
                &File::create(motion_cap_resources.submotion_path.clone()).into_diagnostic()?,
                &motion_cap_resources.currentmotion,
            )
            .into_diagnostic()?;
            motion_cap_resources.new_motion_init = false;
            motion_cap_resources.motion_counter += 1;
        }
    }
    if head_button.rear.is_tapped() {
        if motion_cap_resources.new_motion_init {
            println!("Keyframe recorded:");
            println!("{:?}", naostate.position.clone());
            motion_cap_resources.currentmotion.keyframes.push(Movement {
                target_position: naostate.position.clone(),
                duration: Duration::from_secs(1),
            });
        } else {
            println!(
                "No motion recording active, press the middle headbutton to initialize a movement."
            )
        }
    }

    Ok(())
}
