use nidhogg::{
    types::{ArmJoints, FillExt, HeadJoints, JointArray, LegJoints},
    NaoControlMessage,
};

use crate::{nao, prelude::*};

const STIFFNESS_UNSTIFF: f32 = -1.;

type JointValue = f32;

/// A module providing the motion arbiter.
///
/// All systems that want to set joint values using the motion arbiter, should be executed after
/// [`update_nao_control_message`].
///
/// This module provides the following resources to the application:
/// - [`MotionArbiter`]
pub struct MotionArbiterModule;

impl Module for MotionArbiterModule {
    fn initialize(self, app: App) -> Result<App> {
        app.add_system(update_nao_control_message.before(nao::write_hardware_info))
            .init_resource::<MotionArbiter>()
    }
}

#[system]
pub fn update_nao_control_message(
    control_message: &mut NaoControlMessage,
    motion_manager: &mut MotionArbiter,
) -> Result<()> {
    control_message.position = motion_manager.to_joint_positions();
    control_message.stiffness = motion_manager.to_joint_stiffnesses();

    motion_manager.clear_priorities();

    Ok(())
}

#[derive(Default)]
struct JointSettings<T> {
    joints_position: T,
    joints_stiffness: T,
    priority: Option<Priority>,
}

/// Arbit the motions of multiple modules requesting motions at the same time.
///
/// Modules can request motions through the motion arbiter with a given priority.
/// Each cycle, the motion arbiter will update the [`NaoControlMessage`] with the motions that have the highest
/// priorties.
/// If multiple motion request with the same priority are made, the first request will be chosen.
#[derive(Default)]
pub struct MotionArbiter {
    leg_settings: JointSettings<LegJoints<JointValue>>,
    arm_settings: JointSettings<ArmJoints<JointValue>>,
    head_settings: JointSettings<HeadJoints<JointValue>>,
}

impl MotionArbiter {
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

    fn clear_priorities(&mut self) {
        self.leg_settings.priority = None;
        self.arm_settings.priority = None;
        self.head_settings.priority = None;
    }

    /// Sets the joint position and stifnes of the leg joints.
    ///
    /// The joint positions are degrees in radians.
    ///
    /// The joint stifness should be between 0 and 1, where 1 is maximum stiffness, and 0 minimum
    /// stiffness. A value of `-1` will disable the stiffness altogether.
    pub fn set_legs(
        &mut self,
        joint_positions: LegJoints<JointValue>,
        joint_stiffness: LegJoints<JointValue>,
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

    /// Sets the joint position and stifnes of the arm joints.
    ///
    /// The joint positions are degrees in radians.
    ///
    /// The joint stifness should be between 0 and 1, where 1 is maximum stiffness, and 0 minimum
    /// stiffness. A value of `-1` will disable the stiffness altogether.
    pub fn set_arms(
        &mut self,
        joint_positions: ArmJoints<JointValue>,
        joint_stiffness: ArmJoints<JointValue>,
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

    /// Sets the joint position and stifnes of the head joints.
    ///
    /// The joint positions are degrees in radians.
    ///
    /// The joint stifness should be between 0 and 1, where 1 is maximum stiffness, and 0 minimum
    /// stiffness. A value of `-1` will disable the stiffness altogether.
    pub fn set_head(
        &mut self,
        joint_positions: HeadJoints<JointValue>,
        joint_stiffness: HeadJoints<JointValue>,
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

    /// Disable the stiffness of the legs.
    pub fn unstiff_legs(&mut self, priority: Priority) -> &mut Self {
        self.set_legs(
            self.leg_settings.joints_position.clone(),
            LegJoints::fill(STIFFNESS_UNSTIFF),
            priority,
        )
    }

    /// Disable the stiffness of the legs.
    pub fn unstiff_arms(&mut self, priority: Priority) -> &mut Self {
        self.set_arms(
            self.arm_settings.joints_position.clone(),
            ArmJoints::fill(STIFFNESS_UNSTIFF),
            priority,
        )
    }

    /// Disable the stiffness of the legs.
    pub fn unstiff_head(&mut self, priority: Priority) -> &mut Self {
        self.set_head(
            self.head_settings.joints_position.clone(),
            HeadJoints::fill(STIFFNESS_UNSTIFF),
            priority,
        )
    }

    fn to_joint_positions(&self) -> JointArray<JointValue> {
        JointArray::builder()
            .leg_joints(self.leg_settings.joints_position.clone())
            .arm_joints(self.arm_settings.joints_position.clone())
            .head_joints(self.head_settings.joints_position.clone())
            .build()
    }

    fn to_joint_stiffnesses(&self) -> JointArray<JointValue> {
        JointArray::builder()
            .leg_joints(self.leg_settings.joints_stiffness.clone())
            .arm_joints(self.arm_settings.joints_stiffness.clone())
            .head_joints(self.head_settings.joints_stiffness.clone())
            .build()
    }
}

/// Priority order for the motion arbiter commands.
///
/// Priories are in the range [0, 100].
pub enum Priority {
    /// Has priority `10`.
    Low,
    /// Has priority `30`.
    Medium,
    /// Has priority `60`.
    High,
    /// Has priority `90`.
    Critical,
    /// Custom priority, should be in range [0, 100].
    Custom(u32),
}

impl Priority {
    fn priority_value(&self) -> u32 {
        match self {
            Priority::Low => 10,
            Priority::Medium => 30,
            Priority::High => 60,
            Priority::Critical => 90,
            Priority::Custom(value) => {
                assert!(value <= &100u32);
                *value
            }
        }
    }
}
