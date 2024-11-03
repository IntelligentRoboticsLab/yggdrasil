//! Contains the masses of each link of the robot in kilograms, along with the center of mass (`CoM`) of each
//! link relative to the origin of the robot frame.
//!
//! The origin is the center of the robot's torso, the x-axis vectors forward, the y-axis vectors left,
//! and the z-axis vectors up.
use crate::kinematics::spaces::*;
use nalgebra as na;
use spatial::{types::Vector3, Space, SpaceOver};

/// The mass and center of mass of a link.
#[derive(Debug, Clone)]
pub struct RobotMass<S: Space + SpaceOver<na::Vector3<f32>>> {
    /// Mass of the link in kilograms.
    pub mass: f32,
    /// Center of mass of the link relative to the origin of the robot frame.
    pub center: Vector3<S>,
}

/// Mass and `CoM` of the torso.
pub const TORSO: RobotMass<Torso> = RobotMass {
    mass: 1.0496,
    center: Vector3::new(na::vector![0.0, 0.0, 0.0]),
};

/// Mass and `CoM` of the neck.
pub const NECK: RobotMass<Neck> = RobotMass {
    mass: 0.07842,
    center: Vector3::new(na::vector![-0.00001, 0.0, -0.02742]),
};

/// Mass and `CoM` of the head.
pub const HEAD: RobotMass<Head> = RobotMass {
    mass: 0.65937,
    center: Vector3::new(na::vector![0.00109, 0.00146, 0.05719]),
};

/// Mass and `CoM` of the left.
pub const LEFT_SHOULDER: RobotMass<Shoulder<Left>> = RobotMass {
    mass: 0.09304,
    center: Vector3::new(na::vector![-0.00165, -0.02663, 0.00014]),
};

/// Mass and `CoM` of the left upper arm.
pub const LEFT_UPPER_ARM: RobotMass<UpperArm<Left>> = RobotMass {
    mass: 0.15777,
    center: Vector3::new(na::vector![0.02455, 0.00563, 0.0033]),
};

/// Mass and `CoM` of the left elbow.
pub const LEFT_ELBOW: RobotMass<Elbow<Left>> = RobotMass {
    mass: 0.06483,
    center: Vector3::new(na::vector![-0.02744, 0.0, -0.00014]),
};

/// Mass and `CoM` of the left forearm.
pub const LEFT_FOREARM: RobotMass<Forearm<Left>> = RobotMass {
    mass: 0.07761,
    center: Vector3::new(na::vector![0.02556, 0.00281, 0.00076]),
};

/// Mass and `CoM` of the left wrist.
pub const LEFT_WRIST: RobotMass<Wrist<Left>> = RobotMass {
    mass: 0.18533,
    center: Vector3::new(na::vector![0.03434, -0.00088, 0.00308]),
};

/// Mass and `CoM` of the right shoulder.
pub const RIGHT_SHOULDER: RobotMass<Shoulder<Right>> = RobotMass {
    mass: 0.09304,
    center: Vector3::new(na::vector![-0.00165, 0.02663, 0.00014]),
};

/// Mass and `CoM` of the right upper arm.
pub const RIGHT_UPPER_ARM: RobotMass<UpperArm<Right>> = RobotMass {
    mass: 0.15777,
    center: Vector3::new(na::vector![0.02455, -0.00563, 0.0033]),
};

/// Mass and `CoM` of the right elbow.
pub const RIGHT_ELBOW: RobotMass<Elbow<Right>> = RobotMass {
    mass: 0.06483,
    center: Vector3::new(na::vector![-0.02744, 0.0, -0.00014]),
};

/// Mass and `CoM` of the right forearm.
pub const RIGHT_FOREARM: RobotMass<Forearm<Right>> = RobotMass {
    mass: 0.07761,
    center: Vector3::new(na::vector![0.02556, -0.00281, 0.00076]),
};

/// Mass and `CoM` of the right wrist.
pub const RIGHT_WRIST: RobotMass<Wrist<Right>> = RobotMass {
    mass: 0.18533,
    center: Vector3::new(na::vector![0.03434, 0.00088, 0.00308]),
};

/// Mass and `CoM` of the left hip.
pub const LEFT_PELVIS: RobotMass<Pelvis<Left>> = RobotMass {
    mass: 0.06981,
    center: Vector3::new(na::vector![-0.00781, -0.01114, 0.02661]),
};

/// Mass and `CoM` of the left thigh.
pub const LEFT_HIP: RobotMass<Hip<Left>> = RobotMass {
    mass: 0.14053,
    center: Vector3::new(na::vector![-0.01549, 0.00029, -0.00515]),
};

/// Mass and `CoM` of the left thigh.
pub const LEFT_THIGH: RobotMass<Thigh<Left>> = RobotMass {
    mass: 0.38968,
    center: Vector3::new(na::vector![0.00138, 0.00221, -0.05373]),
};

/// Mass and `CoM` of the left tibia.
pub const LEFT_TIBIA: RobotMass<Tibia<Left>> = RobotMass {
    mass: 0.30142,
    center: Vector3::new(na::vector![0.00453, 0.00225, -0.04936]),
};

/// Mass and `CoM` of the left ankle.
pub const LEFT_ANKLE: RobotMass<Ankle<Left>> = RobotMass {
    mass: 0.13416,
    center: Vector3::new(na::vector![0.00045, 0.00029, 0.00685]),
};

/// Mass and `CoM` of the left foot.
pub const LEFT_FOOT: RobotMass<Foot<Left>> = RobotMass {
    mass: 0.17184,
    center: Vector3::new(na::vector![0.02542, 0.0033, -0.03239]),
};

/// Mass and `CoM` of the right pelvis.
pub const RIGHT_PELVIS: RobotMass<Pelvis<Right>> = RobotMass {
    mass: 0.06981,
    center: Vector3::new(na::vector![-0.00781, 0.01114, 0.02661]),
};

/// Mass and `CoM` of the right thigh.
pub const RIGHT_HIP: RobotMass<Hip<Right>> = RobotMass {
    mass: 0.14053,
    center: Vector3::new(na::vector![-0.01549, -0.00029, -0.00515]),
};

/// Mass and `CoM` of the right thigh.
pub const RIGHT_THIGH: RobotMass<Thigh<Right>> = RobotMass {
    mass: 0.38968,
    center: Vector3::new(na::vector![0.00138, -0.00221, -0.05373]),
};

/// Mass and `CoM` of the right tibia.
pub const RIGHT_TIBIA: RobotMass<Tibia<Right>> = RobotMass {
    mass: 0.30142,
    center: Vector3::new(na::vector![0.00453, -0.00225, -0.04936]),
};

/// Mass and `CoM` of the right ankle.
pub const RIGHT_ANKLE: RobotMass<Ankle<Right>> = RobotMass {
    mass: 0.13416,
    center: Vector3::new(na::vector![0.00045, -0.00029, 0.00685]),
};

/// Mass and `CoM` of the right foot.
pub const RIGHT_FOOT: RobotMass<Foot<Right>> = RobotMass {
    mass: 0.17184,
    center: Vector3::new(na::vector![0.02542, -0.0033, -0.03239]),
};

/// Total mass of the robot.
pub const TOTAL_MASS: f32 = TORSO.mass
    + NECK.mass
    + HEAD.mass
    + LEFT_SHOULDER.mass
    + LEFT_UPPER_ARM.mass
    + LEFT_ELBOW.mass
    + LEFT_FOREARM.mass
    + LEFT_WRIST.mass
    + RIGHT_SHOULDER.mass
    + RIGHT_UPPER_ARM.mass
    + RIGHT_ELBOW.mass
    + RIGHT_FOREARM.mass
    + RIGHT_WRIST.mass
    + LEFT_PELVIS.mass
    + LEFT_HIP.mass
    + LEFT_THIGH.mass
    + LEFT_TIBIA.mass
    + LEFT_ANKLE.mass
    + LEFT_FOOT.mass
    + RIGHT_PELVIS.mass
    + RIGHT_HIP.mass
    + RIGHT_THIGH.mass
    + RIGHT_TIBIA.mass
    + RIGHT_ANKLE.mass
    + RIGHT_FOOT.mass;
