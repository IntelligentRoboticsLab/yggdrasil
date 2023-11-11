use nalgebra::{vector, Isometry2};

use crate::{Cov, State, UkfState};

#[derive(Debug, Clone, Copy)]
pub struct Pose2D {
    pub isometry: Isometry2<f32>,
}

impl Pose2D {
    pub fn new(isometry: Isometry2<f32>) -> Self {
        Self { isometry }
    }

    fn to_vector(self) -> State<f32, 3> {
        vector![
            self.isometry.translation.x,
            self.isometry.translation.y,
            self.isometry.rotation.angle()
        ]
    }
}

#[derive(Debug)]
pub struct UkfPose2D {
    // 2n + 1 = 7
    ukf_state: UkfState<f32, 3, 7>,
}

impl UkfPose2D {
    pub fn new(x_0: Pose2D) -> Self {
        Self {
            ukf_state: UkfState::new(0.001, 2.0, 0.0, x_0.to_vector()),
        }
    }

    pub fn with_params(alpha: f32, beta: f32, kappa: f32, x_0: Pose2D) -> Self {
        Self {
            ukf_state: UkfState::new(alpha, beta, kappa, x_0.to_vector()),
        }
    }
}

impl UkfPose2D {
    #[must_use]
    pub fn to_pose(&self) -> Pose2D {
        let mean = self.ukf_state.gaussian.mean;
        let isometry = Isometry2::new(vector![mean.x, mean.y], mean.z);
        Pose2D { isometry }
    }

    pub fn predict_odometry(
        &mut self,
        // pose change relative to last pose
        offset: &Pose2D,
        transition_noise: &Cov<f32, 3>,
    ) {
        self.ukf_state.predict(
            |sigma_point| {
                // rotate odometry from robot to field frame
                let Isometry2 {
                    translation,
                    rotation,
                } = offset.isometry;

                let robot_odometry = rotation * translation.vector;
                let update = vector![robot_odometry.x, robot_odometry.y, rotation.angle()];

                // updated sigma points with odometry offset
                sigma_point + update
            },
            transition_noise,
        );
    }

    pub fn update<const MEASUREMENT_DIM: usize>(
        &mut self,
        measurement_function: impl Fn(State<f32, 3>) -> State<f32, MEASUREMENT_DIM>,
        measurement: State<f32, MEASUREMENT_DIM>,
        measurement_noise: Cov<f32, MEASUREMENT_DIM>,
    ) {
        self.ukf_state
            .update(measurement_function, measurement, measurement_noise);
    }
}
