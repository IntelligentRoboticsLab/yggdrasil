use crate::{
    kinematics::FootOffset,
    walk::{
        engine::{Side, StepOffsets, WalkCommand},
        smoothing, WalkingEngineConfig,
    },
};
use std::time::Duration;

use super::{WalkContext, WalkState, WalkStateKind};

#[derive(Debug, Clone)]
pub struct WalkingState {
    swing_foot: Side,
    phase_time: Duration,
    next_foot_switch: Duration,
    phase_time_on_last_phase_end: Duration,
    previous_step_offsets: StepOffsets,
    max_swing_lift_last_step: f32,
}

impl WalkingState {
    pub fn new(config: &WalkingEngineConfig, left: FootOffset, right: FootOffset) -> Self {
        Self {
            swing_foot: Side::Left,
            phase_time: Duration::ZERO,
            phase_time_on_last_phase_end: config.base_step_period,
            next_foot_switch: config.base_step_period,
            previous_step_offsets: StepOffsets {
                swing: left,
                support: right,
            },
            max_swing_lift_last_step: 0.0,
        }
    }
}

impl WalkState for WalkingState {
    fn next_state(self, context: WalkContext) -> WalkStateKind {
        let phase_time = self.phase_time + context.dt;
        // this is the linear progression of this step, a value from 0 to 1 which describes the progress of the current step.
        let linear_time =
            (phase_time.as_secs_f32() / self.next_foot_switch.as_secs_f32()).clamp(0.0, 1.0);

        context
            .dbg
            .log_scalar_f32("/foot/linear_time", linear_time)
            .unwrap();

        context
            .dbg
            .log_scalar_f32(
                "/foot/parabolic_time",
                smoothing::parabolic_step(linear_time),
            )
            .unwrap();

        // if self.next_foot_switch.as_secs_f32() <= 0.0 {
        if self.next_foot_switch.as_secs_f32() <= 0.0 {
            tracing::info!("early switch!");
            return self.next_walk_state(
                context.dt,
                linear_time,
                &context,
                self.previous_step_offsets.swing,
                self.previous_step_offsets.support,
                context.config.base_foot_lift,
            );
        }

        let command = self.adjust_command(context.config, context.walk_command.clone());
        // compute the max foot height, for moving forward/left we slightly increase the max height
        let max_foot_height = context.config.base_foot_lift
            + (command.forward.abs() * 0.01)
            + (command.left.abs() * 0.02);
        // compute the swing foot height for the current cycle in the step phase
        let swing_foot_height = max_foot_height * smoothing::parabolic_return(linear_time);
        let swing_foot = self.swing_foot;

        // compute the offsets for the support and swing feet
        let support_offset =
            self.compute_support_offset(&command, context.config, &swing_foot, phase_time);
        let swing_offset = self.compute_swing_offset(
            &command,
            context.config,
            &swing_foot,
            swing_foot_height,
            linear_time,
        );

        self.next_walk_state(
            context.dt,
            linear_time,
            &context,
            swing_offset,
            support_offset,
            max_foot_height,
        )
    }

    fn get_foot_offsets(&self) -> (FootOffset, FootOffset) {
        let swing_foot = self.swing_foot;
        let previous_step = &self.previous_step_offsets;
        match swing_foot {
            Side::Right => (previous_step.swing, previous_step.support),
            Side::Left => (previous_step.support, previous_step.swing),
        }
    }

    fn swing_foot(&self) -> Side {
        self.swing_foot
    }
}

impl WalkingState {
    fn adjust_command(&self, config: &WalkingEngineConfig, command: WalkCommand) -> WalkCommand {
        // we multiply the walk command values by 2T as there's two steps per period
        let period = 2.0 * config.base_step_period.as_secs_f32();

        let new = WalkCommand {
            forward: command.forward * period,
            left: command.left * period * 0.82,
            turn: command.turn * period * 1.43,
        };
        tracing::info!(
            "period: {period}, old: {}, new: {}",
            command.forward,
            new.forward
        );
        new
    }
    fn next_walk_state(
        &self,
        dt: Duration,
        linear_time: f32,
        context: &WalkContext,
        swing_offset: FootOffset,
        support_offset: FootOffset,
        max_foot_height: f32,
    ) -> WalkStateKind {
        let mut next_swing_foot = self.swing_foot;
        let mut phase_time = self.phase_time + dt;
        let mut phase_time_on_last_phase_end = self.phase_time_on_last_phase_end;
        let mut next_foot_switch = self.next_foot_switch;

        let mut previous_step = StepOffsets {
            swing: swing_offset,
            support: support_offset,
        };

        // figure out whether the support foot has changed
        let has_support_foot_changed = linear_time > 0.75 && self.has_support_foot_changed(context);

        // if the support foot has in fact changed, we should update the relevant parameters
        if has_support_foot_changed {
            next_swing_foot = self.swing_foot.next();

            // reset phase
            next_foot_switch = context.config.base_step_period;
            phase_time_on_last_phase_end = phase_time;
            phase_time = Duration::ZERO;

            // Switch these around, so the offsets are maintained throughout the walk cycle
            previous_step.support = swing_offset;
            previous_step.swing = support_offset;

            previous_step.swing.left = -previous_step.support.left;
        }

        WalkStateKind::Walking(WalkingState {
            swing_foot: next_swing_foot,
            phase_time,
            phase_time_on_last_phase_end,
            next_foot_switch,
            previous_step_offsets: previous_step,
            max_swing_lift_last_step: max_foot_height,
        })
    }

    fn has_support_foot_changed(&self, context: &WalkContext) -> bool {
        let left_foot_pressure = context.fsr.left_foot.sum();
        let right_foot_pressure = context.fsr.right_foot.sum();
        (match self.swing_foot {
            Side::Left => left_foot_pressure,
            Side::Right => right_foot_pressure,
        }) > context.config.cop_pressure_threshold
    }

    fn compute_swing_offset(
        &self,
        walk_command: &WalkCommand,
        config: &WalkingEngineConfig,
        side: &Side,
        foot_height: f32,
        linear_time: f32,
    ) -> FootOffset {
        let FootOffset {
            forward: forward_t0,
            left: left_t0,
            turn: turn_t0,
            hip_height: _,
            lift: _,
        } = self.previous_step_offsets.swing;
        let parabolic_time = smoothing::parabolic_step(linear_time);

        let turn_multiplier = match side {
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

    fn compute_support_offset(
        &self,
        walk_command: &WalkCommand,
        config: &WalkingEngineConfig,
        side: &Side,
        phase_time: Duration,
    ) -> FootOffset {
        let FootOffset {
            forward: forward_t0,
            left: left_t0,
            turn: turn_t0,
            hip_height: _,
            lift: _,
        } = self.previous_step_offsets.support;

        let turn_multiplier = match side {
            Side::Left => -1.0 / 3.0,
            Side::Right => 1.0 / 3.0,
        };

        let linear_time = phase_time.as_secs_f32() / config.base_step_period.as_secs_f32();
        let support_foot_lift = self.max_swing_lift_last_step
            * smoothing::parabolic_return(
                ((self.phase_time_on_last_phase_end.as_secs_f32() + phase_time.as_secs_f32())
                    / config.base_step_period.as_secs_f32())
                .clamp(0.0, 1.0),
            );

        FootOffset {
            forward: forward_t0
                + (-(walk_command.forward) * config.com_multiplier - forward_t0) * linear_time,
            left: left_t0 + (-walk_command.left / 2.0 - left_t0) * linear_time,
            turn: turn_t0 + (-walk_command.turn * turn_multiplier - turn_t0) * linear_time,
            hip_height: config.hip_height,
            lift: support_foot_lift,
        }
    }
}
