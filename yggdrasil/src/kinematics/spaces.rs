use nalgebra as na;
use spatial::{Space, SpaceOver};

pub struct Left;
pub struct Right;

macro_rules! impl_space {
    ($space:ident) => {
        pub struct $space;
        impl Space for $space {}
        impl SpaceOver<na::Point3<f32>> for $space {}
        impl SpaceOver<na::Vector3<f32>> for $space {}
        impl SpaceOver<na::Isometry3<f32>> for $space {}
    };
    ($space:ident<T>) => {
        pub struct $space<T: ?Sized>(std::marker::PhantomData<T>);
        impl Space for $space<Left> {}
        impl Space for $space<Right> {}
        impl SpaceOver<na::Point3<f32>> for $space<Left> {}
        impl SpaceOver<na::Point3<f32>> for $space<Right> {}
        impl SpaceOver<na::Vector3<f32>> for $space<Left> {}
        impl SpaceOver<na::Vector3<f32>> for $space<Right> {}
        impl SpaceOver<na::Isometry3<f32>> for $space<Left> {}
        impl SpaceOver<na::Isometry3<f32>> for $space<Right> {}
    };
}

macro_rules! impl_spaces {
    ($($a:ident),*; $($b:ident<T>,)*) => {
        $(impl_space!{$a})*
        $(impl_space!{$b<T>})*
    }
}

impl_spaces! {
    Head,
    Neck,
    Torso,
    Robot,
    Ground;
    Shoulder<T>,
    UpperArm<T>,
    Elbow<T>,
    Forearm<T>,
    Wrist<T>,
    Pelvis<T>,
    Hip<T>,
    Thigh<T>,
    Tibia<T>,
    Ankle<T>,
    Foot<T>,
    Sole<T>,
    Toe<T>,
}

pub type LeftShoulder = Shoulder<Left>;
pub type LeftUpperArm = UpperArm<Left>;
pub type LeftElbow = Elbow<Left>;
pub type LeftForearm = Forearm<Left>;
pub type LeftWrist = Wrist<Left>;
pub type LeftPelvis = Pelvis<Left>;
pub type LeftHip = Hip<Left>;
pub type LeftThigh = Thigh<Left>;
pub type LeftTibia = Tibia<Left>;
pub type LeftAnkle = Ankle<Left>;
pub type LeftFoot = Foot<Left>;
pub type LeftSole = Sole<Left>;
pub type LeftToe = Toe<Left>;

pub type RightShoulder = Shoulder<Right>;
pub type RightUpperArm = UpperArm<Right>;
pub type RightElbow = Elbow<Right>;
pub type RightForearm = Forearm<Right>;
pub type RightWrist = Wrist<Right>;
pub type RightPelvis = Pelvis<Right>;
pub type RightHip = Hip<Right>;
pub type RightThigh = Thigh<Right>;
pub type RightTibia = Tibia<Right>;
pub type RightAnkle = Ankle<Right>;
pub type RightFoot = Foot<Right>;
pub type RightSole = Sole<Right>;
pub type RightToe = Toe<Right>;
