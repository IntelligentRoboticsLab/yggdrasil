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
        if engine.is_walking() {
            let position = pose.robot_to_world(&Point2::new(dist, 0.));

            planner.add_dynamic_obstacle(DynamicObstacle {
                obs: Obstacle::new(position.x, position.y, 0.5),
                ttl: Instant::now() + Duration::from_secs_f32(5.),
            }, 0.25);
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
}

#[derive(Debug, Default)]
struct SonarSide {
    history: VecDeque<f32>,
}

impl Sonar {
    pub fn obstruction(&self) -> Option<f32> {
        self.obstruction
    }

    fn detect_obstruction(&self) -> Option<f32> {
        let (a, b, c, d) = if self.obstruction.is_some() {
            (0.60, 0.50, 0.75, 0.60)
        } else {
            (0.50, 0.80, 0.95, 0.50)
        };

        let left = self.left.ratio(a);
        let right = self.right.ratio(a);

        let dist = self.left.mean(d).min(self.right.mean(d));


        if left >= b && right >= b {
            Some(dist)
        } else if left.max(right) >= c {
            Some(dist)
        } else {
            None
        }
    }

    fn update_from_values(&mut self, values: &SonarValues) {
        self.left.update_from_value(values.left);
        self.right.update_from_value(values.right);

        let obstruction = self.detect_obstruction();

        self.obstruction = obstruction;
    }
}

impl SonarSide {
    const HISTORY_SIZE: usize = 24;

    fn history(&self, threshold: f32) -> impl '_ + Iterator<Item = f32> + Clone {
        self.history.iter().copied().filter(move |x| *x <= threshold)
    }

    fn ratio(&self, threshold: f32) -> f32 {
        self.history(threshold).count() as f32 / Self::HISTORY_SIZE as f32
    }

    fn mean(&self, threshold: f32) -> f32 {
        let history = self.history(threshold);
        history.clone().sum::<f32>() / history.count() as f32
        
    }

    fn update_from_value(&mut self, value: f32) {
        if self.history.len() >= Self::HISTORY_SIZE {
            self.history.pop_front();
        }

        self.history.push_back(value);
    }
}
