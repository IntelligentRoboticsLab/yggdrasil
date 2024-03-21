use nidhogg::types::{ArmJoints, FillExt, HeadJoints, JointArray, LegJoints};

use crate::{nao, prelude::*};

pub struct MotionManagerModule;

const STIFFNESS_UNSTIFF: f32 = -1.;

/// A module providing the motion manager.
///
/// All systems that want to set joint values using the motion manager, should be executed after
/// [`clear_priorities`].
///
/// This module provides the following resources to the application:
/// - [`MotionManager`]
impl Module for MotionManagerModule {
    fn initialize(self, app: App) -> Result<App> {
        app.add_system(clear_priorities.after(nao::write_hardware_info))
            .init_resource::<MotionManager>()
    }
}

#[system]
pub fn clear_priorities(motion_manager: &mut MotionManager) -> Result<()> {
    motion_manager.clear_priorities();

    Ok(())
}

pub type JointDataType = f32;

#[derive(Default)]
struct JointSettings<T> {
    joints_position: T,
    joints_stiffness: T,
    priority: Option<Priority>,
}

#[derive(Default)]
pub struct MotionManager {
    leg_settings: JointSettings<LegJoints<JointDataType>>,
    arm_settings: JointSettings<ArmJoints<JointDataType>>,
    head_settings: JointSettings<HeadJoints<JointDataType>>,
}

impl MotionManager {
    fn set_settings<T>(
        current_settings: &mut JointSettings<T>,
        joint_positions: T,
        joint_stiffness: T,
        priority: Priority,
    ) {
        if current_settings
            .priority
            .as_ref()
            .is_some_and(|current_priority| {
                current_priority.priority_value() >= priority.priority_value()
            })
        {
            return;
        }

        current_settings.joints_position = joint_positions;
        current_settings.joints_stiffness = joint_stiffness;
        current_settings.priority = Some(priority);
    }

    pub fn clear_priorities(&mut self) {
        self.leg_settings.priority = None;
        self.arm_settings.priority = None;
        self.head_settings.priority = None;
    }

    pub fn set_legs(
        &mut self,
        joint_positions: LegJoints<JointDataType>,
        joint_stiffness: LegJoints<JointDataType>,
        priority: Priority,
    ) -> &mut Self {
        Self::set_settings(
            &mut self.leg_settings,
            joint_positions,
            joint_stiffness,
            priority,
        );

        self
    }

    pub fn set_arms(
        &mut self,
        joint_positions: ArmJoints<JointDataType>,
        joint_stiffness: ArmJoints<JointDataType>,
        priority: Priority,
    ) -> &mut Self {
        Self::set_settings(
            &mut self.arm_settings,
            joint_positions,
            joint_stiffness,
            priority,
        );

        self
    }

    pub fn set_head(
        &mut self,
        joint_positions: HeadJoints<JointDataType>,
        joint_stiffness: HeadJoints<JointDataType>,
        priority: Priority,
    ) -> &mut Self {
        Self::set_settings(
            &mut self.head_settings,
            joint_positions,
            joint_stiffness,
            priority,
        );

        self
    }

    pub fn unstiff_legs(&mut self, priority: Priority) -> &mut Self {
        self.set_legs(
            self.leg_settings.joints_position.clone(),
            LegJoints::fill(STIFFNESS_UNSTIFF),
            priority,
        )
    }

    pub fn unstiff_arms(&mut self, priority: Priority) -> &mut Self {
        self.set_arms(
            self.arm_settings.joints_position.clone(),
            ArmJoints::<f32>::fill(STIFFNESS_UNSTIFF),
            priority,
        )
    }

    pub fn unstiff_head(&mut self, priority: Priority) -> &mut Self {
        self.set_head(
            self.head_settings.joints_position.clone(),
            HeadJoints::fill(STIFFNESS_UNSTIFF),
            priority,
        )
    }

    pub fn to_joint_positions(&self) -> JointArray<JointDataType> {
        JointArray::builder()
            .leg_joints(self.leg_settings.joints_position.clone())
            .arm_joints(self.arm_settings.joints_position.clone())
            .head_joints(self.head_settings.joints_position.clone())
            .build()
    }

    pub fn to_joint_stiffnesses(&self) -> JointArray<JointDataType> {
        JointArray::builder()
            .leg_joints(self.leg_settings.joints_stiffness.clone())
            .arm_joints(self.arm_settings.joints_stiffness.clone())
            .head_joints(self.head_settings.joints_stiffness.clone())
            .build()
    }
}

pub enum Priority {
    Low,
    Medium,
    High,
    Critical,
    Custom(u32),
}

impl Priority {
    fn priority_value(&self) -> u32 {
        match self {
            Priority::Low => 10,
            Priority::Medium => 30,
            Priority::High => 60,
            Priority::Critical => 90,
            Priority::Custom(value) => *value,
        }
    }
}
