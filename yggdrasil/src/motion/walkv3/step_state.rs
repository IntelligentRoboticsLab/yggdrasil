use std::time::Duration;

use nalgebra::{Isometry3, Point3, Rotation3, Translation, Vector, Vector3};

use crate::motion::walk::smoothing::{self, parabolic_step};

use super::{feet::Feet, step::PlannedStep};

#[derive(Default, Debug, Clone)]
pub struct StepState {
    pub duration: Duration,
    pub planned_step: PlannedStep,
}

impl StepState {
    pub fn update(&mut self, delta_time: Duration) {
        self.duration += delta_time;
    }

    pub fn compute_feet(&self) -> Feet {
        let normalized_time =
            self.duration.as_secs_f32() / self.planned_step.duration.as_secs_f32();

        let swing = self.compute_swing(normalized_time);
        let support = self.compute_support(normalized_time);

        Feet { swing, support }
    }

    pub fn compute_swing(&self, normalized_time: f32) -> Isometry3<f32> {
        let parabolic_time = smoothing::parabolic_step(normalized_time);
        let parabolic_return = smoothing::parabolic_return(normalized_time);

        let start = self.planned_step.start.swing;
        let end = self.planned_step.end.swing;

        println!("start: {start}, end: {end}");

        let swing_position = start
            .translation
            .vector
            .lerp(&end.translation.vector, parabolic_time);

        let start = start.rotation;
        let target = end.rotation;

        let max_rotation_speed = 50000.0;
        let max_rotation_delta = self.duration.as_secs_f32() * max_rotation_speed;

        let (roll, pitch, yaw) = start.rotation_to(&target).euler_angles();
        let interpolated_roll = roll.clamp(-max_rotation_delta, max_rotation_delta);
        let interpolated_pitch = pitch.clamp(-max_rotation_delta, max_rotation_delta);
        let interpolated_yaw = lerp(0.0, yaw, normalized_time);
        let interpolated =
            Rotation3::from_euler_angles(interpolated_roll, interpolated_pitch, interpolated_yaw);

        let swing_rotation = interpolated * start;

        Isometry3::from_parts(
            Translation::from(Vector3::new(
                swing_position.x,
                swing_position.y,
                self.planned_step.foot_height * parabolic_return,
            )),
            swing_rotation,
        )
    }

    pub fn compute_support(&self, normalized_time: f32) -> Isometry3<f32> {
        let start = self.planned_step.start.support;
        let end = self.planned_step.end.support;

        let support_position = start
            .translation
            .vector
            .lerp(&end.translation.vector, normalized_time);

        let start = start.rotation;
        let target = end.rotation;

        let max_rotation_speed = 50000.0;
        let max_rotation_delta = self.duration.as_secs_f32() * max_rotation_speed;

        let (roll, pitch, yaw) = start.rotation_to(&target).euler_angles();
        let interpolated_roll = roll.clamp(-max_rotation_delta, max_rotation_delta);
        let interpolated_pitch = pitch.clamp(-max_rotation_delta, max_rotation_delta);
        let interpolated_yaw = lerp(0.0, yaw, normalized_time);
        let interpolated =
            Rotation3::from_euler_angles(interpolated_roll, interpolated_pitch, interpolated_yaw);

        let support_rotation = interpolated * start;

        Isometry3::from_parts(
            Translation::from(Vector3::new(support_position.x, support_position.y, 0.0)),
            support_rotation,
        )
    }
}

fn lerp(from: f32, to: f32, t: f32) -> f32 {
    from + t * (to - from)
}
