use std::time::Duration;

use nidhogg::types::{FillExt, JointArray, LeftLegJoints, RightLegJoints};

use crate::{
    kinematics::{self, FootOffset},
    walk::{
        engine::{Side, StepOffsets, WalkCommand},
        smoothing,
    },
};

use super::{WalkContext, WalkState, WalkStateKind};

#[derive(Debug)]
pub struct WalkingState {
    pub swing_foot: Side,
    pub phase_time: Duration,
    pub next_foot_switch: Duration,
    pub previous_step: StepOffsets,
}

impl Default for WalkingState {
    fn default() -> Self {
        Self {
            swing_foot: Side::Right,
            phase_time: Duration::ZERO,
            next_foot_switch: Duration::ZERO,
            previous_step: StepOffsets::default(),
        }
    }
}

impl WalkState for WalkingState {
    fn next_state(&self, context: WalkContext) -> WalkStateKind {
        let linear_time = self.linear_time(&context);

        if self.next_foot_switch.as_secs_f32() <= 0.0 {
            return self.next_walk_state(
                context.dt,
                linear_time,
                &context,
                self.previous_step.swing,
                self.previous_step.support,
            );
        }

        let WalkCommand {
            forward,
            left,
            turn: _,
        } = context.walk_command;
        // compute the max foot height, for moving forward/left we slightly increase the max height
        let max_foot_height =
            context.config.base_foot_lift + (forward.abs() * 0.01) + (left.abs() * 0.02);
        // compute the swing foot height for the current cycle in the step phase
        let swing_foot_height = max_foot_height * smoothing::parabolic_return(linear_time);

        let swing_foot = self.swing_foot;

        // compute the offsets for the support and swing feet
        let support_offset = self.compute_support_offset(&context, linear_time);
        let swing_offset = self.compute_swing_offset(&context, swing_foot_height, linear_time);

        let next_state = self.next_walk_state(
            context.dt,
            linear_time,
            &context,
            swing_offset,
            support_offset,
        );

        let (left_foot, right_foot) = match swing_foot {
            Side::Left => (swing_offset, support_offset),
            Side::Right => (support_offset, swing_offset),
        };

        // the shoulder pitch is "approximated" by taking the opposite direction multiplied by a constant.
        // this results in a swing motion that moves in the opposite direction as the foot.
        let balancing_config = &context.config.balancing;
        let left_shoulder_pitch = -left_foot.forward * balancing_config.arm_swing_multiplier;
        let right_shoulder_pitch = -right_foot.forward * balancing_config.arm_swing_multiplier;

        let (mut left_leg_joints, mut right_leg_joints) =
            kinematics::inverse::leg_angles(&left_foot, &right_foot);

        // Balance adjustment
        let balance_adjustment =
            context.filtered_gyro.y() * balancing_config.filtered_gyro_y_multiplier;
        if self.next_foot_switch.as_millis() > 0 {
            match swing_foot {
                Side::Left => {
                    right_leg_joints.ankle_pitch += balance_adjustment;
                }
                Side::Right => {
                    left_leg_joints.ankle_pitch += balance_adjustment;
                }
            }
        } else {
            right_leg_joints.ankle_pitch += balance_adjustment;
            left_leg_joints.ankle_pitch += balance_adjustment;
        }

        let stiffness = 1.0;
        context.control_message.stiffness = JointArray::<f32>::builder()
            .left_shoulder_pitch(stiffness)
            .left_shoulder_roll(stiffness)
            .right_shoulder_pitch(stiffness)
            .right_shoulder_roll(stiffness)
            .head_pitch(1.0)
            .head_yaw(1.0)
            .left_leg_joints(LeftLegJoints::fill(stiffness))
            .right_leg_joints(RightLegJoints::fill(stiffness))
            .build();

        context.control_message.position = JointArray::<f32>::builder()
            .left_shoulder_pitch(90f32.to_radians() + left_shoulder_pitch)
            .left_shoulder_roll(7f32.to_radians())
            .right_shoulder_pitch(90f32.to_radians() + right_shoulder_pitch)
            .right_shoulder_roll(-7f32.to_radians())
            .left_leg_joints(left_leg_joints.clone())
            .right_leg_joints(right_leg_joints.clone())
            .build();
        next_state
    }
}

impl WalkingState {
    pub fn next_walk_state(
        &self,
        dt: Duration,
        linear_time: f32,
        context: &WalkContext,
        swing_offset: FootOffset,
        support_offset: FootOffset,
    ) -> WalkStateKind {
        let mut next_swing_foot = self.swing_foot;
        let mut phase_time = self.phase_time + dt;
        let mut next_foot_switch = self.next_foot_switch;

        let mut previous_step = self.previous_step.clone();
        // figure out whether the support foot has changed
        let has_support_foot_changed = linear_time > 0.75 && self.has_support_foot_changed(context);

        // if the support foot has in fact changed, we should update the relevant parameters
        if has_support_foot_changed {
            next_swing_foot = self.swing_foot.next();

            // reset phase
            next_foot_switch = context.config.base_step_period;
            phase_time = Duration::ZERO;

            // Switch these around, so the offsets are maintained throughout the walk cycle
            previous_step.support = swing_offset;
            previous_step.swing = support_offset;

            previous_step.swing.left = -previous_step.support.left;
        }

        WalkStateKind::Walking(WalkingState {
            swing_foot: next_swing_foot,
            phase_time,
            next_foot_switch,
            previous_step,
        })
    }

    pub fn has_support_foot_changed(&self, context: &WalkContext) -> bool {
        let left_foot_pressure = context.fsr.left_foot.sum();
        let right_foot_pressure = context.fsr.right_foot.sum();
        (match self.swing_foot {
            Side::Left => left_foot_pressure,
            Side::Right => right_foot_pressure,
        }) > context.config.cop_pressure_threshold
    }

    pub fn compute_swing_offset(
        &self,
        context: &WalkContext,
        foot_height: f32,
        linear_time: f32,
    ) -> FootOffset {
        let walk_command = &context.walk_command;
        let config = &context.config;
        let FootOffset {
            forward: forward_t0,
            left: left_t0,
            turn: turn_t0,
            hip_height: _,
            lift: _,
        } = self.previous_step.swing;
        let parabolic_time = smoothing::parabolic_step(linear_time);

        let turn_multiplier = match self.swing_foot {
            Side::Left => -2.0 / 3.0,
            Side::Right => 2.0 / 3.0,
        };
        FootOffset {
            forward: forward_t0
                + (walk_command.forward * config.com_multiplier - forward_t0) * parabolic_time,
            left: left_t0 + (walk_command.left / 2.0 - left_t0) * parabolic_time,
            turn: turn_t0 + (walk_command.turn * turn_multiplier - turn_t0) * parabolic_time,
            hip_height: config.hip_height,
            lift: foot_height,
        }
    }

    pub fn compute_support_offset(&self, context: &WalkContext, linear_time: f32) -> FootOffset {
        let walk_command = &context.walk_command;
        let config = &context.config;
        let FootOffset {
            forward: forward_t0,
            left: left_t0,
            turn: turn_t0,
            hip_height: _,
            lift: _,
        } = self.previous_step.support;

        let turn_multiplier = match self.swing_foot {
            Side::Left => -1.0 / 3.0,
            Side::Right => 1.0 / 3.0,
        };

        FootOffset {
            forward: forward_t0
                + (-walk_command.forward * config.com_multiplier - forward_t0) * linear_time,
            left: left_t0 + (-walk_command.left / 2.0 - left_t0) * linear_time,
            turn: turn_t0 + (-walk_command.turn * turn_multiplier - turn_t0) * linear_time,
            hip_height: config.hip_height,
            lift: 0.0,
        }
    }

    pub fn linear_time(&self, ctx: &WalkContext) -> f32 {
        let phase_time = self.phase_time + ctx.dt;
        // this is the linear progression of this step, a value from 0 to 1 which describes the progress of the current step.
        (phase_time.as_secs_f32() / self.next_foot_switch.as_secs_f32()).clamp(0.0, 1.0)
    }
}
