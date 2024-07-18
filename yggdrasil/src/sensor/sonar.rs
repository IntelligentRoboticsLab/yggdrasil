use std::{collections::VecDeque, time::{Duration, Instant}};

use crate::{
    localization::RobotPose,
    motion::{path_finding::Obstacle, step_planner::{DynamicObstacle, StepPlanner}, walk::engine::WalkingEngine},
    nao::manager::{NaoManager, Priority},
    prelude::*,
};
use nalgebra::Point2;
use nidhogg::{
    types::{color, FillExt, RightEye, SonarValues},
    NaoState,
};

/// A module offering structured wrappers for sonar, derived from the raw [`NaoState`].
///
/// This module provides the following resources to the application:
/// - [`SonarValues`]
pub struct SonarSensor;

impl Module for SonarSensor {
    fn initialize(self, app: App) -> Result<App> {
        app.add_staged_system(SystemStage::Sensor, sonar_sensor)
            .init_resource::<Sonar>()
    }
}

#[system]
fn sonar_sensor(
    nao_state: &NaoState,
    engine: &WalkingEngine,
    planner: &mut StepPlanner,
    pose: &RobotPose,
    nao: &mut NaoManager,
    sonar: &mut Sonar,
) -> Result<()> {
    sonar.update_from_values(&nao_state.sonar);

    if let Some(dist) = sonar.obstruction() {
        if engine.is_walking() && sonar.positive_edge() {
            let position = pose.robot_to_world(&Point2::new(dist.max(0.1) + 1., 0.));

            planner.add_dynamic_obstacle(DynamicObstacle {
                obs: Obstacle::new(position.x, position.y, 1.),
                ttl: Instant::now() + Duration::from_secs_f32(7.5),
            });
        }

        nao.set_right_eye_led(RightEye::fill(color::f32::RED), Priority::High);
    } else {
        nao.set_right_eye_led(RightEye::fill(color::f32::BLUE), Priority::High);
    }

    Ok(())
}

#[derive(Debug, Default)]
pub struct Sonar {
    left: SonarSide,
    right: SonarSide,
    obstruction: Option<f32>,
    positive_edge: bool,
}

#[derive(Debug, Default)]
struct SonarSide {
    accum: (usize, f32),
    history: VecDeque<Option<f32>>,
}

impl Sonar {
    pub fn obstruction(&self) -> Option<f32> {
        self.obstruction
    }

    pub fn positive_edge(&self) -> bool {
        self.positive_edge
    }

    fn detect_obstruction(&self) -> Option<f32> {
        let left = self.left.ratio();
        let right = self.right.ratio();
        let both = left + right;

        let dist = self.left.mean().min(self.right.mean());

        if left > 0.3 && right > 0.3 {
            (both > 0.6).then_some(dist)
        } else {
            (both > 0.8 && dist < 0.8).then_some(dist)
        }
    }

    fn update_from_values(&mut self, values: &SonarValues) {
        self.left.update_from_value(values.left);
        self.right.update_from_value(values.right);

        let obstruction = self.detect_obstruction();

        self.positive_edge = obstruction.is_some() && self.obstruction.is_none();
        self.obstruction = obstruction;
    }
}

impl SonarSide {
    const HISTORY_SIZE: usize = 128;

    fn ratio(&self) -> f32 {
        self.accum.0 as f32 / Self::HISTORY_SIZE as f32
    }

    fn mean(&self) -> f32 {
        if self.accum.0 == 0 {
            f32::INFINITY
        } else {
            self.accum.1 / self.accum.0 as f32
        }
    }

    fn update_from_value(&mut self, value: f32) {
        let value = (value < 5.).then_some(value);

        if let Some(value) = value {
            self.accum.0 += 1;
            self.accum.1 += value;
        }

        if self.history.len() >= Self::HISTORY_SIZE {
            if let Some(old) = self.history.pop_front().unwrap() {
                self.accum.0 -= 1;

                if self.accum.0 == 0 {
                    self.accum.1 = 0.;
                } else {
                    self.accum.1 -= old;
                }
            }
        }

        self.history.push_back(value);
    }
}
