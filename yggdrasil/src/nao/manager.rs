use std::{
    cmp::Ordering,
    ops::Not,
    time::{Duration, Instant},
};

use nidhogg::{
    types::{
        color, ArmJoints, FillExt, HeadJoints, JointArray, LeftEar, LeftEye, LegJoints, RgbF32,
        RightEar, RightEye, Skull,
    },
    NaoControlMessage,
};

use crate::prelude::*;

const STIFFNESS_UNSTIFF: f32 = -1.0;

type JointValue = f32;

/// A module providing the nao manager.
///
/// All systems that want to set joint- or LED values using the nao manager, should be executed before
/// [`finalize`].
///
/// This module provides the following resources to the application:
/// - [`NaoManager`]
pub struct NaoManagerModule;

impl Module for NaoManagerModule {
    fn initialize(self, app: App) -> Result<App> {
        app.add_staged_system(SystemStage::Finalize, finalize)
            .init_resource::<NaoManager>()
    }
}

#[system]
pub fn finalize(control_message: &mut NaoControlMessage, manager: &mut NaoManager) -> Result<()> {
    control_message.position = manager.make_joint_positions();
    control_message.stiffness = manager.make_joint_stiffnesses();

    control_message.left_ear = manager.led_left_ear.value.clone();
    control_message.right_ear = manager.led_right_ear.value.clone();
    control_message.chest = manager.led_chest.value.color();
    control_message.left_eye = manager.led_left_eye.value.clone();
    control_message.right_eye = manager.led_right_eye.value.clone();
    control_message.left_foot = manager.led_left_foot.value;
    control_message.right_foot = manager.led_right_foot.value;
    control_message.skull = manager.led_skull.value.clone();

    manager.clear_priorities();

    Ok(())
}

#[derive(Default)]
struct JointSettings<T> {
    joints_position: T,
    joints_stiffness: T,
    priority: Option<Priority>,
}

enum ChestBlink {
    Static {
        color: RgbF32,
    },
    Blinking {
        color: RgbF32,
        interval: Duration,
        on: bool,
        start: Instant,
    },
}

impl ChestBlink {
    pub fn color(&mut self) -> RgbF32 {
        match self {
            ChestBlink::Static { color } => *color,
            ChestBlink::Blinking {
                color,
                interval,
                on,
                start,
            } => {
                if start.elapsed() > *interval {
                    *on = on.not();
                    *start = Instant::now();
                }

                if *on {
                    *color
                } else {
                    color::f32::EMPTY
                }
            }
        }
    }
}

impl Default for ChestBlink {
    fn default() -> Self {
        ChestBlink::Static {
            color: color::f32::EMPTY,
        }
    }
}

#[derive(Default)]
struct LedSettings<T> {
    value: T,
    priority: Option<Priority>,
}

/// Manager that handles the requests of multiple systems changing the desired nao state at the same time.
///
/// Modules can request through the nao manager with a given priority.
/// Each cycle, the nao manager will update the [`NaoControlMessage`] with the requests that have the highest
/// priorties.
/// If multiple requests with the same priority are made, the first request will be prioritized.
#[derive(Default)]
pub struct NaoManager {
    leg_settings: JointSettings<LegJoints<JointValue>>,
    arm_settings: JointSettings<ArmJoints<JointValue>>,
    head_settings: JointSettings<HeadJoints<JointValue>>,

    led_left_ear: LedSettings<LeftEar>,
    led_right_ear: LedSettings<RightEar>,
    led_chest: LedSettings<ChestBlink>,
    led_left_eye: LedSettings<LeftEye>,
    led_right_eye: LedSettings<RightEye>,
    led_left_foot: LedSettings<RgbF32>,
    led_right_foot: LedSettings<RgbF32>,
    led_skull: LedSettings<Skull>,
}

impl NaoManager {
    fn set_joint_settings<T>(
        current_settings: &mut JointSettings<T>,
        joint_positions: T,
        joint_stiffness: T,
        priority: Priority,
    ) {
        if current_settings
            .priority
            .as_ref()
            .is_some_and(|current_priority| current_priority >= &priority)
        {
            return;
        }

        current_settings.joints_position = joint_positions;
        current_settings.joints_stiffness = joint_stiffness;
        current_settings.priority = Some(priority);
    }

    fn set_led_settings<T>(current_settings: &mut LedSettings<T>, leds: T, priority: Priority) {
        if current_settings
            .priority
            .as_ref()
            .is_some_and(|current_priority| current_priority >= &priority)
        {
            return;
        }

        current_settings.value = leds;
        current_settings.priority = Some(priority);
    }

    fn clear_priorities(&mut self) {
        self.leg_settings.priority = None;
        self.arm_settings.priority = None;
        self.head_settings.priority = None;

        self.led_left_ear.priority = None;
        self.led_right_ear.priority = None;
        self.led_chest.priority = None;
        self.led_left_eye.priority = None;
        self.led_right_eye.priority = None;
        self.led_left_foot.priority = None;
        self.led_right_foot.priority = None;
        self.led_skull.priority = None;
    }

    /// Try to set all the joint position and stiffness of the legs, arms and head.
    /// The joint positions are angles in radians.
    ///
    /// # Notes
    /// - It is possible that one or all of the groups are not set, if another request
    ///   has a higher priority.
    /// - The joint stiffness should be between 0 and 1, where 1 is maximum stiffness, and 0 minimum
    ///   stiffness. A value of `-1` will disable the stiffness altogether.
    pub fn set_all(
        &mut self,
        initial_joint_positions: JointArray<JointValue>,
        head_stiffness: HeadJoints<JointValue>,
        arm_stiffness: ArmJoints<JointValue>,
        leg_stiffness: LegJoints<JointValue>,
        priority: Priority,
    ) -> &mut Self {
        self.set_legs(
            initial_joint_positions.leg_joints(),
            leg_stiffness,
            priority,
        )
        .set_arms(
            initial_joint_positions.arm_joints(),
            arm_stiffness,
            priority,
        )
        .set_head(
            initial_joint_positions.head_joints(),
            head_stiffness,
            priority,
        )
    }

    /// Sets the joint position and stiffness of the leg joints.
    ///
    /// The joint positions are degrees in radians.
    ///
    /// The joint stiffness should be between 0 and 1, where 1 is maximum stiffness, and 0 minimum
    /// stiffness. A value of `-1` will disable the stiffness altogether.
    pub fn set_legs(
        &mut self,
        joint_positions: LegJoints<JointValue>,
        joint_stiffness: LegJoints<JointValue>,
        priority: Priority,
    ) -> &mut Self {
        Self::set_joint_settings(
            &mut self.leg_settings,
            joint_positions,
            joint_stiffness,
            priority,
        );

        self
    }

    /// Sets the joint position and stiffness of the arm joints.
    ///
    /// The joint positions are degrees in radians.
    ///
    /// The joint stiffness should be between 0 and 1, where 1 is maximum stiffness, and 0 minimum
    /// stiffness. A value of `-1` will disable the stiffness altogether.
    pub fn set_arms(
        &mut self,
        joint_positions: ArmJoints<JointValue>,
        joint_stiffness: ArmJoints<JointValue>,
        priority: Priority,
    ) -> &mut Self {
        Self::set_joint_settings(
            &mut self.arm_settings,
            joint_positions,
            joint_stiffness,
            priority,
        );

        self
    }

    /// Sets the joint position and stiffness of the head joints.
    ///
    /// The joint positions are degrees in radians.
    ///
    /// The joint stiffness should be between 0 and 1, where 1 is maximum stiffness, and 0 minimum
    /// stiffness. A value of `-1` will disable the stiffness altogether.
    pub fn set_head(
        &mut self,
        joint_positions: HeadJoints<JointValue>,
        joint_stiffness: HeadJoints<JointValue>,
        priority: Priority,
    ) -> &mut Self {
        Self::set_joint_settings(
            &mut self.head_settings,
            joint_positions,
            joint_stiffness,
            priority,
        );

        self
    }

    /// Disable all motors in the legs.
    pub fn unstiff_legs(&mut self, priority: Priority) -> &mut Self {
        self.set_legs(
            self.leg_settings.joints_position.clone(),
            LegJoints::fill(STIFFNESS_UNSTIFF),
            priority,
        )
    }

    /// Disable all motors in the arms.
    pub fn unstiff_arms(&mut self, priority: Priority) -> &mut Self {
        self.set_arms(
            self.arm_settings.joints_position.clone(),
            ArmJoints::fill(STIFFNESS_UNSTIFF),
            priority,
        )
    }

    /// Disable all motors in the head.
    pub fn unstiff_head(&mut self, priority: Priority) -> &mut Self {
        self.set_head(
            self.head_settings.joints_position.clone(),
            HeadJoints::fill(STIFFNESS_UNSTIFF),
            priority,
        )
    }

    pub fn set_left_ear_led(&mut self, left_ear: LeftEar, priority: Priority) -> &mut Self {
        Self::set_led_settings(&mut self.led_left_ear, left_ear, priority);

        self
    }

    pub fn set_right_ear_led(&mut self, right_ear: RightEar, priority: Priority) -> &mut Self {
        Self::set_led_settings(&mut self.led_right_ear, right_ear, priority);

        self
    }

    pub fn set_chest_led(&mut self, chest: RgbF32, priority: Priority) -> &mut Self {
        Self::set_led_settings(
            &mut self.led_chest,
            ChestBlink::Static { color: chest },
            priority,
        );

        self
    }

    pub fn set_chest_blink_led(
        &mut self,
        chest: RgbF32,
        interval: Duration,
        priority: Priority,
    ) -> &mut Self {
        match self.led_chest.value {
            ChestBlink::Static { .. } => {
                Self::set_led_settings(
                    &mut self.led_chest,
                    ChestBlink::Blinking {
                        color: chest,
                        interval,
                        on: false,
                        start: Instant::now(),
                    },
                    priority,
                );
            }
            ChestBlink::Blinking { on, start, .. } => {
                Self::set_led_settings(
                    &mut self.led_chest,
                    ChestBlink::Blinking {
                        color: chest,
                        interval,
                        on,
                        start,
                    },
                    priority,
                );
            }
        }

        self
    }

    pub fn set_left_eye_led(&mut self, left_eye: LeftEye, priority: Priority) -> &mut Self {
        Self::set_led_settings(&mut self.led_left_eye, left_eye, priority);

        self
    }

    pub fn set_right_eye_led(&mut self, right_eye: RightEye, priority: Priority) -> &mut Self {
        Self::set_led_settings(&mut self.led_right_eye, right_eye, priority);

        self
    }

    pub fn set_left_foot_led(&mut self, left_foot: RgbF32, priority: Priority) -> &mut Self {
        Self::set_led_settings(&mut self.led_left_foot, left_foot, priority);

        self
    }

    pub fn set_right_foot_led(&mut self, right_foot: RgbF32, priority: Priority) -> &mut Self {
        Self::set_led_settings(&mut self.led_right_foot, right_foot, priority);

        self
    }

    pub fn set_skull_led(&mut self, skull: Skull, priority: Priority) -> &mut Self {
        Self::set_led_settings(&mut self.led_skull, skull, priority);

        self
    }

    fn make_joint_positions(&self) -> JointArray<JointValue> {
        JointArray::builder()
            .leg_joints(self.leg_settings.joints_position.clone())
            .arm_joints(self.arm_settings.joints_position.clone())
            .head_joints(self.head_settings.joints_position.clone())
            .build()
    }

    fn make_joint_stiffnesses(&self) -> JointArray<JointValue> {
        JointArray::builder()
            .leg_joints(self.leg_settings.joints_stiffness.clone())
            .arm_joints(self.arm_settings.joints_stiffness.clone())
            .head_joints(self.head_settings.joints_stiffness.clone())
            .build()
    }
}

/// Priority order for the nao manager commands.
///
/// Priories are in the range [0, 100].
#[derive(Default, Clone, Copy, Debug)]
pub enum Priority {
    /// Has priority `10`.
    #[default]
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

impl PartialEq for Priority {
    fn eq(&self, other: &Self) -> bool {
        self.priority_value() == other.priority_value()
    }
}

impl PartialOrd for Priority {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.priority_value().partial_cmp(&other.priority_value())
    }
}
