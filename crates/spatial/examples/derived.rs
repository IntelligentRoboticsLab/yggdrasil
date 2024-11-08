use nalgebra as na;
use spatial::types::{Isometry3, Point3};
use spatial::Transform;

macro_rules! kinematics_spaces {
    {$($space:ident,)*} => {
        $(
            struct $space;
            impl ::spatial::Space for $space {}
            impl ::spatial::SpaceOver<na::Point3<f32>> for $space {}
        )*
    };
}

kinematics_spaces! {
    Pelvis,
    Torso,
    Head,
    LeftArm,
    RightArm,
    LeftLeg,
    RightLeg,
}

#[derive(Default, spatial::Transform)]
struct Kinematics {
    pelvis_to_torso: Isometry3<Pelvis, Torso>,
    torso_to_head: Isometry3<Torso, Head>,
    torso_to_left_arm: Isometry3<Torso, LeftArm>,
    torso_to_right_arm: Isometry3<Torso, RightArm>,
    pelvis_to_left_leg: Isometry3<Pelvis, LeftLeg>,
    pelvis_to_right_leg: Isometry3<Pelvis, RightLeg>,
}

fn main() {
    let kinematics = Kinematics::default();

    let p1: Point3<LeftArm> = Point3::default();
    let p2: Point3<RightLeg> = kinematics.transform(&p1);

    println!("{p1:?} becomes {p2:?}");
}
