use nalgebra as na;

use niflheim::types::*;
use niflheim::{Space, SpaceOver, Transform};

struct LocalSpace;

impl Space for LocalSpace {}
impl SpaceOver<na::Point3<f32>> for LocalSpace {}

struct WorldSpace;

impl Space for WorldSpace {}
impl SpaceOver<na::Point3<f32>> for WorldSpace {}

fn main() {
    let local_to_world: Isometry3<LocalSpace, WorldSpace> =
        na::Isometry3::new(na::vector![1., 2., 3.], na::vector![0., 0., 0.]).into();

    let x: Point3<LocalSpace> = na::point![1., 0., 0.].into();
    let y: Point3<WorldSpace> = local_to_world.transform(&x);

    println!("{:?} is {:?}", x, y);
}
