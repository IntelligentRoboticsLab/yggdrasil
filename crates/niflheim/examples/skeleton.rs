use niflheim::skeleton::{Root, Skeleton};
use niflheim::types::{Isometry3, Point3};
use niflheim::{link, parent, propagate_link, Space, SpaceOver};

use nalgebra as na;

struct S0;
struct S1;
struct S2;
struct S3;

impl Space for S0 {}
impl SpaceOver<na::Point3<f32>> for S0 {}
impl Space for S1 {}
impl SpaceOver<na::Point3<f32>> for S1 {}
impl Space for S2 {}
impl SpaceOver<na::Point3<f32>> for S2 {}
impl Space for S3 {}
impl SpaceOver<na::Point3<f32>> for S3 {}

struct MySkeleton {
    s1_to_s0: Isometry3<S1, S0>,
    s2_to_s0: Isometry3<S2, S0>,
    s3_to_s2: Isometry3<S3, S2>,
}

impl Skeleton for MySkeleton {}

impl<S1, S2> Root<na::Point3<f32>, S1, na::Point3<f32>, S2> for MySkeleton {
    type T = na::Point3<f32>;
    type S = S0;
}

parent!(MySkeleton, S0);
parent!(MySkeleton, S2);

link!(MySkeleton, na::Isometry3<f32>, S1, S0, s1_to_s0);
link!(MySkeleton, na::Isometry3<f32>, S2, S0, s2_to_s0);
link!(MySkeleton, na::Isometry3<f32>, S3, S2, s3_to_s2);

propagate_link!(MySkeleton, S2, S0);

fn main() {
    let sk = MySkeleton {
        s1_to_s0: na::Isometry3::translation(1., 0., 0.).into(),
        s2_to_s0: na::Isometry3::translation(0., 1., 0.).into(),
        s3_to_s2: na::Isometry3::translation(0., 0., 1.).into(),
    };

    let x: Point3<S2> = na::Point3::new(0., 0., 0.).into();

    let y: Point3<S1> = sk.transform(&x);
    let z: Point3<S3> = sk.transform_via::<_, _, _, _, _, S2>(&x);

    println!("{x:?} -> {y:?} and {z:?}");
}
