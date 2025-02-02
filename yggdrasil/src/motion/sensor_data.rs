//! Visualize sensor data using Rerun.

use bevy::prelude::*;
use nalgebra::{Point2, Vector2};
use nidhogg::types::{Fsr, FsrFoot};
use rerun::{components::Scalar, Color, ComponentBatch, EntityPath, TimeColumn};

use crate::{
    core::debug::DebugContext,
    nao::{CenterOfMass, CenterOfPressure, Cycle, ZeroMomentPoint},
    sensor::imu::IMUValues,
};

use super::walkv4::SwingFoot;

/// Plugin for visualizing sensor data using Rerun.
pub(super) struct VisualizeSensorDataPlugin;

impl Plugin for VisualizeSensorDataPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostStartup, setup_visualization)
            .add_systems(
                PostUpdate,
                (
                    // visualize_gyroscope,
                    // visualize_accelerometer,
                    // visualize_com,
                    // visualize_center_of_pressure,
                    // visualize_zero_moment_point,
                    visualize_fsr,
                ),
            );
    }
}

type IMUReading = (f32, f32, f32);

fn setup_visualization(dbg: DebugContext) {
    setup_color(&dbg, "gyro/x", Color::from_rgb(255, 0, 0));
    setup_color(&dbg, "gyro/y", Color::from_rgb(0, 255, 0));
    setup_color(&dbg, "gyro/z", Color::from_rgb(0, 0, 255));

    setup_color(&dbg, "accel/x", Color::from_rgb(255, 0, 0));
    setup_color(&dbg, "accel/y", Color::from_rgb(0, 255, 0));
    setup_color(&dbg, "accel/z", Color::from_rgb(0, 0, 255));

    setup_color(&dbg, "com/x", Color::from_rgb(255, 0, 0));
    setup_color(&dbg, "com/y", Color::from_rgb(0, 255, 0));
    setup_color(&dbg, "com/z", Color::from_rgb(0, 0, 255));

    setup_color(&dbg, "left_cop/x", Color::from_rgb(255, 0, 0));
    setup_color(&dbg, "left_cop/y", Color::from_rgb(0, 255, 0));

    setup_color(&dbg, "right_cop/x", Color::from_rgb(255, 0, 0));
    setup_color(&dbg, "right_cop/y", Color::from_rgb(0, 255, 0));

    setup_color(&dbg, "zmp/x", Color::from_rgb(255, 0, 0));
    setup_color(&dbg, "zmp/y", Color::from_rgb(0, 255, 0));
}

fn setup_color(dbg: &DebugContext, path: impl Into<EntityPath>, color: Color) {
    dbg.log_component_batches(path, true, [&color as &dyn ComponentBatch]);
}

fn visualize_gyroscope(
    dbg: DebugContext,
    mut buffer: Local<Vec<(Cycle, IMUReading)>>,
    cycle: Res<Cycle>,
    imu: Res<IMUValues>,
) {
    if buffer.len() >= 20 {
        let (cycles, ((x_readings, y_readings), z_readings)): (Vec<_>, ((Vec<_>, Vec<_>), Vec<_>)) =
            buffer
                .iter()
                .copied()
                .map(|(cycle, reading)| {
                    (
                        cycle.0 as i64,
                        ((reading.0 as f64, reading.1 as f64), reading.2 as f64),
                    )
                })
                .unzip();

        let x_scalars: Vec<Scalar> = x_readings.into_iter().map(Into::into).collect();
        let y_scalars: Vec<Scalar> = y_readings.into_iter().map(Into::into).collect();
        let z_scalars: Vec<Scalar> = z_readings.into_iter().map(Into::into).collect();

        let timeline = TimeColumn::new_sequence("cycle", cycles);
        dbg.send_columns("gyro/x", [timeline.clone()], [&x_scalars as _]);
        dbg.send_columns("gyro/y", [timeline.clone()], [&y_scalars as _]);
        dbg.send_columns("gyro/z", [timeline], [&z_scalars as _]);

        buffer.clear();
    } else {
        buffer.push((
            cycle.clone(),
            (imu.gyroscope.x, imu.gyroscope.y, imu.gyroscope.z),
        ));
    }
}

fn visualize_accelerometer(
    dbg: DebugContext,
    mut buffer: Local<Vec<(Cycle, IMUReading)>>,
    cycle: Res<Cycle>,
    imu: Res<IMUValues>,
) {
    if buffer.len() >= 20 {
        let (cycles, ((x_readings, y_readings), z_readings)): (Vec<_>, ((Vec<_>, Vec<_>), Vec<_>)) =
            buffer
                .iter()
                .copied()
                .map(|(cycle, reading)| {
                    (
                        cycle.0 as i64,
                        ((reading.0 as f64, reading.1 as f64), reading.2 as f64),
                    )
                })
                .unzip();

        let x_scalars: Vec<Scalar> = x_readings.into_iter().map(Into::into).collect();
        let y_scalars: Vec<Scalar> = y_readings.into_iter().map(Into::into).collect();
        let z_scalars: Vec<Scalar> = z_readings.into_iter().map(Into::into).collect();

        let timeline = TimeColumn::new_sequence("cycle", cycles);
        dbg.send_columns("accel/x", [timeline.clone()], [&x_scalars as _]);
        dbg.send_columns("accel/y", [timeline.clone()], [&y_scalars as _]);
        dbg.send_columns("accel/z", [timeline], [&z_scalars as _]);

        buffer.clear();
    } else {
        buffer.push((
            cycle.clone(),
            (
                imu.accelerometer.x,
                imu.accelerometer.y,
                imu.accelerometer.z,
            ),
        ));
    }
}

fn visualize_com(
    dbg: DebugContext,
    mut buffer: Local<Vec<(Cycle, IMUReading)>>,
    cycle: Res<Cycle>,
    com: Res<CenterOfMass>,
) {
    if buffer.len() >= 20 {
        let (cycles, ((x_readings, y_readings), z_readings)): (Vec<_>, ((Vec<_>, Vec<_>), Vec<_>)) =
            buffer
                .iter()
                .copied()
                .map(|(cycle, reading)| {
                    (
                        cycle.0 as i64,
                        ((reading.0 as f64, reading.1 as f64), reading.2 as f64),
                    )
                })
                .unzip();

        let x_scalars: Vec<Scalar> = x_readings.into_iter().map(Into::into).collect();
        let y_scalars: Vec<Scalar> = y_readings.into_iter().map(Into::into).collect();
        let z_scalars: Vec<Scalar> = z_readings.into_iter().map(Into::into).collect();

        let timeline = TimeColumn::new_sequence("cycle", cycles);
        dbg.send_columns("com/x", [timeline.clone()], [&x_scalars as _]);
        dbg.send_columns("com/y", [timeline.clone()], [&y_scalars as _]);
        dbg.send_columns("com/z", [timeline], [&z_scalars as _]);

        buffer.clear();
    } else {
        buffer.push((
            cycle.clone(),
            (com.position.x, com.position.y, com.position.z),
        ));
    }
}

fn visualize_center_of_pressure(
    dbg: DebugContext,
    mut left_buffer: Local<Vec<(Cycle, Vector2<f32>)>>,
    mut right_buffer: Local<Vec<(Cycle, Vector2<f32>)>>,
    cycle: Res<Cycle>,
    cop: Res<CenterOfPressure>,
) {
    if left_buffer.len() >= 20 && right_buffer.len() >= 20 {
        let (left_cycles, (left_x_readings, left_y_readings)): (Vec<_>, (Vec<_>, Vec<_>)) =
            left_buffer
                .iter()
                .copied()
                .map(|(cycle, reading)| (cycle.0 as i64, (reading.x as f64, reading.y as f64)))
                .unzip();
        let (right_cycles, (right_x_readings, right_y_readings)): (Vec<_>, (Vec<_>, Vec<_>)) =
            right_buffer
                .iter()
                .copied()
                .map(|(cycle, reading)| (cycle.0 as i64, (reading.x as f64, reading.y as f64)))
                .unzip();

        let x_scalars: Vec<Scalar> = left_x_readings.into_iter().map(Into::into).collect();
        let y_scalars: Vec<Scalar> = left_y_readings.into_iter().map(Into::into).collect();

        let timeline = TimeColumn::new_sequence("cycle", left_cycles);
        dbg.send_columns("left_cop/x", [timeline.clone()], [&x_scalars as _]);
        dbg.send_columns("left_cop/y", [timeline], [&y_scalars as _]);

        let x_scalars: Vec<Scalar> = right_x_readings.into_iter().map(Into::into).collect();
        let y_scalars: Vec<Scalar> = right_y_readings.into_iter().map(Into::into).collect();

        let timeline = TimeColumn::new_sequence("cycle", right_cycles);
        dbg.send_columns("right_cop/x", [timeline.clone()], [&x_scalars as _]);
        dbg.send_columns("right_cop/y", [timeline], [&y_scalars as _]);

        left_buffer.clear();
        right_buffer.clear();
    } else {
        left_buffer.push((cycle.clone(), cop.left));
        right_buffer.push((cycle.clone(), cop.right));
    }
}

fn visualize_zero_moment_point(
    dbg: DebugContext,
    mut buffer: Local<Vec<(Cycle, Point2<f32>)>>,
    cycle: Res<Cycle>,
    zmp: Res<ZeroMomentPoint>,
) {
    if buffer.len() >= 20 {
        let (cycles, (x_readings, y_readings)): (Vec<_>, (Vec<_>, Vec<_>)) = buffer
            .iter()
            .copied()
            .map(|(cycle, reading)| (cycle.0 as i64, (reading.x as f64, reading.y as f64)))
            .unzip();

        let x_scalars: Vec<Scalar> = x_readings.into_iter().map(Into::into).collect();
        let y_scalars: Vec<Scalar> = y_readings.into_iter().map(Into::into).collect();

        let timeline = TimeColumn::new_sequence("cycle", cycles);
        dbg.send_columns("zmp/x", [timeline.clone()], [&x_scalars as _]);
        dbg.send_columns("zmp/y", [timeline], [&y_scalars as _]);

        buffer.clear();
    } else {
        buffer.push((cycle.clone(), zmp.point));
    }
}

fn visualize_fsr(
    dbg: DebugContext,
    mut buffer: Local<Vec<(Cycle, Fsr)>>,
    cycle: Res<Cycle>,
    fsr: Res<Fsr>,
    swing_foot: Res<SwingFoot>,
) {
    if buffer.len() >= 20 {
        let (cycles, (left_fsr, (right_fsr, swing))): (Vec<_>, (Vec<_>, (Vec<_>, Vec<_>))) = buffer
            .iter()
            .cloned()
            .map(|(cycle, reading)| {
                (
                    cycle.0 as i64,
                    (reading.left_foot, (reading.right_foot, swing_foot.clone())),
                )
            })
            .unzip();

        let left_foot: Vec<Scalar> = left_fsr
            .iter()
            .map(|f| f.sum() as f64)
            .map(Into::into)
            .collect();
        let right_foot: Vec<Scalar> = right_fsr
            .iter()
            .map(|f| f.sum() as f64)
            .map(Into::into)
            .collect();

        let swing_foot: Vec<Scalar> = swing
            .iter()
            .map(|f| match **f {
                super::walkv4::Side::Left => 1.0,
                super::walkv4::Side::Right => -1.0,
            })
            .map(Into::into)
            .collect();

        let timeline = TimeColumn::new_sequence("cycle", cycles);
        dbg.send_columns("fsr/left", [timeline.clone()], [&left_foot as _]);
        dbg.send_columns("fsr/right", [timeline.clone()], [&right_foot as _]);
        dbg.send_columns("fsr/swing", [timeline.clone()], [&swing_foot as _]);

        buffer.clear();
    } else {
        buffer.push((cycle.clone(), fsr.clone()));
    }
}
