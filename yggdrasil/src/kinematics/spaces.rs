use nalgebra as na;
use spatial::{Space, SpaceOver};

pub(crate) use super::{Left, Right};

macro_rules! impl_space {
    ($space:ident) => {
        pub struct $space;
        impl Space for $space {}
        impl SpaceOver<na::Point3<f32>> for $space {}
        impl SpaceOver<na::Vector3<f32>> for $space {}
    };
    ($space:ident<T>) => {
        pub struct $space<T>(T);
        impl Space for $space<Left> {}
        impl Space for $space<Right> {}
        impl SpaceOver<na::Point3<f32>> for $space<Left> {}
        impl SpaceOver<na::Point3<f32>> for $space<Right> {}
        impl SpaceOver<na::Vector3<f32>> for $space<Left> {}
        impl SpaceOver<na::Vector3<f32>> for $space<Right> {}
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
    Robot;
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
}
