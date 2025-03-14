use std::{
    cmp::Ordering,
    ops::Not,
    time::{Duration, Instant},
};

use crate::prelude::*;
use bevy::prelude::*;
use nalgebra::UnitQuaternion;
use nidhogg::{
    types::{
        color, ArmJoints, FillExt, HeadJoints, JointArray, LeftEar, LeftEye, LegJoints, RgbF32,
        RightEar, RightEye, Skull,
    },
    NaoControlMessage, NaoState,
};
/// The stiffness constant for the "unstiff"/"floppy" state for robot joints.
const STIFFNESS_UNSTIFF: f32 = -1.0;
/// Stiffness for the hip joints during sitting mode to prevent robot falling over backwards.
const HIP_LOCK_STIFFNESS: f32 = 0.1;
/// The set hip position in sitting mode, where the robot sits and starts.
const HIP_POSITION: f32 = -0.9;

const CYCLES_PER_SECOND: f32 = 82.0;

const SOURCE_TARGET_SIMILARITY_THRESHOLD: f32 = 0.99;

const NEW_TARGET_SIMILARITY_THRESHOLD: f32 = 0.9;

type JointValue = f32;

/// Plugin providing the [`NaoManager`].
///
/// All systems that want to set joint- or LED values using the nao manager, should be executed before
/// [`finalize`].
///
/// This module provides the following resources to the application:
/// - [`NaoManager`]
pub(super) struct NaoManagerPlugin;

impl Plugin for NaoManagerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<NaoManager>()
            .add_systems(PreWrite, finalize);
    }
}

fn finalize(
    mut control_message: ResMut<NaoControlMessage>,
    mut manager: ResMut<NaoManager>,
    state: Res<NaoState>,
) {
    // Update the head target
    manager.head_state = manager.head_state.clone().update(&state);

    // If the head is moving, interpolate the desired head position and set the head joints.
    if let HeadState::Moving {
        source,
        target,
        timestep,
        time_interval: _,
        priority,
        stiffness,
    } = manager.head_state
    {
        let head = source.slerp(&target, timestep);

        manager.set_head(
            HeadJoints::builder()
                .pitch(head.euler_angles().1)
                .yaw(head.euler_angles().2)
                .build(),
            HeadJoints::fill(stiffness),
            priority,
        );
    }

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
}

#[derive(Default, Debug)]
struct JointSettings<T> {
    joints_position: T,
    joints_stiffness: T,
    priority: Option<Priority>,
}

#[derive(Debug)]
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

#[derive(Default, Debug)]
struct LedSettings<T> {
    value: T,
    priority: Option<Priority>,
}

// This enum represents the current state of the head target/motion.
#[derive(Default, Debug, Clone)]
enum HeadState {
    #[default]
    Stationary,
    New {
        target: UnitQuaternion<f32>,
        time_to_target: Duration,
        priority: Priority,
        stiffness: f32,
    },
    Moving {
        source: UnitQuaternion<f32>,
        target: UnitQuaternion<f32>,
        timestep: f32,
        time_interval: f32,
        priority: Priority,
        stiffness: f32,
    },
}

impl HeadState {
    // This function is called every cycle to update the head target depending on the current state of the robot.
    fn update(self, nao_state: &NaoState) -> Self {
        match self {
            HeadState::Stationary => HeadState::Stationary,
            // If the target is new, we start moving towards it.
            HeadState::New {
                target,
                time_to_target,
                priority,
                stiffness,
            } => {
                let source = UnitQuaternion::from_euler_angles(
                    0.0,
                    nao_state.position.head_pitch,
                    nao_state.position.head_yaw,
                );

                let similarity = source.dot(&target);
                if similarity > SOURCE_TARGET_SIMILARITY_THRESHOLD {
                    HeadState::Stationary
                } else {
                    HeadState::Moving {
                        source,
                        target,
                        timestep: 0.0,
                        time_interval: 1.0 / (time_to_target.as_secs_f32() * CYCLES_PER_SECOND),
                        priority,
                        stiffness,
                    }
                }
            }
            // If the target is already moving, we check if we have already reached the target.
            HeadState::Moving {
                source,
                target,
                timestep,
                time_interval,
                priority,
                stiffness,
            } => {
                //
                if timestep >= 1.0 {
                    HeadState::Stationary
                } else {
                    HeadState::Moving {
                        source,
                        target,
                        timestep: timestep + time_interval,
                        time_interval,
                        priority,
                        stiffness,
                    }
                }
            }
        }
    }
}

/// Manager that handles the requests of multiple systems changing the desired nao state at the same time.
///
/// Modules can request through the nao manager with a given priority.
/// Each cycle, the nao manager will update the [`NaoControlMessage`] with the requests that have the highest
/// priorities.
/// If multiple requests with the same priority are made, the first request will be prioritized.
#[derive(Default, Debug, Resource)]
pub struct NaoManager {
    leg_settings: JointSettings<LegJoints<JointValue>>,
    arm_settings: JointSettings<ArmJoints<JointValue>>,
    head_settings: JointSettings<HeadJoints<JointValue>>,

    head_state: HeadState,

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
    pub const HEAD_STIFFNESS: f32 = 0.7;

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

    /// Set the target position for the head.
    pub fn set_head_target(
        &mut self,
        joint_positions: HeadJoints<JointValue>,
        time_to_target: Duration,
        priority: Priority,
        stiffness: f32,
    ) -> &mut Self {
        let new_target =
            UnitQuaternion::from_euler_angles(0.0, joint_positions.pitch, joint_positions.yaw);

        if let HeadState::Moving {
            source,
            target,
            timestep,
            time_interval,
            priority,
            stiffness,
        } = self.head_state
        {
            // If the head is already moving, only set a new target if its sufficiently different from old target.
            // This helps revents the head from stuttering
            let similarity = target.dot(&new_target);
            if similarity < NEW_TARGET_SIMILARITY_THRESHOLD {
                self.head_state = HeadState::New {
                    target: new_target,
                    time_to_target,
                    priority,
                    stiffness,
                };
            } else {
                self.head_state = HeadState::Moving {
                    source,
                    target: new_target,
                    timestep,
                    time_interval,
                    priority,
                    stiffness,
                };
            }
            return self;
        }

        // If the head is not moving, we just set the new target.
        self.head_state = HeadState::New {
            target: new_target,
            time_to_target,
            priority,
            stiffness,
        };

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

    /// Set all joints to unstiff, then lock the hip joints in their seated position
    /// with constant stiffness to avoid falling over.
    pub fn unstiff_sit(&mut self, priority: Priority) -> &mut Self {
        let mut leg_stiffness = LegJoints::fill(STIFFNESS_UNSTIFF);
        leg_stiffness.left_leg.hip_pitch = HIP_LOCK_STIFFNESS;
        leg_stiffness.left_leg.hip_yaw_pitch = HIP_LOCK_STIFFNESS;
        leg_stiffness.right_leg.hip_pitch = HIP_LOCK_STIFFNESS;

        let mut leg_positions = self.leg_settings.joints_position.clone();
        leg_positions.left_leg.hip_pitch = HIP_POSITION;
        leg_positions.right_leg.hip_pitch = HIP_POSITION;

        self.set_legs(leg_positions, leg_stiffness, priority)
    }

    pub fn stiff_sit(&mut self, leg_positions: LegJoints<f32>, priority: Priority) -> &mut Self {
        let leg_stiffness = LegJoints::fill(0.3);
        self.set_legs(leg_positions, leg_stiffness, priority)
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
#[derive(Default, Clone, Copy, Debug, Eq)]
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
    const fn priority_value(self) -> u32 {
        match self {
            Priority::Low => 10,
            Priority::Medium => 30,
            Priority::High => 60,
            Priority::Critical => 90,
            Priority::Custom(value) => {
                assert!(value <= 100u32);
                value
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
