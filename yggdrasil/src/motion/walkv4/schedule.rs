use std::time::Duration;

use bevy::{
    app::MainScheduleOrder,
    ecs::schedule::{
        ExecutorKind, InternedScheduleLabel, LogLevel, ScheduleBuildSettings, ScheduleLabel,
    },
    prelude::*,
};

use crate::kinematics::Kinematics;

use super::{
    config::WalkingEngineConfig,
    feet::FootPositions,
    step::{PlannedStep, Step},
    step_manager::StepManager,
    Side, TORSO_OFFSET,
};

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MotionSet {
    StepPlanning,
    GaitGeneration,
    Balancing,
    Finalize,
}

impl MotionSet {
    /// The order of the motion system sets.
    fn order() -> impl IntoSystemSetConfigs {
        (
            MotionSet::StepPlanning,
            MotionSet::GaitGeneration,
            MotionSet::Balancing,
            MotionSet::Finalize,
        )
            .chain()
    }
}

#[derive(States, Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Gait {
    #[default]
    Sitting,
    Standing,
    Starting,
    Walking,
}

#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub(super) struct StepPlanning;

#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub(super) struct GaitGeneration;

#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub(super) struct Balancing;

#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub(super) struct Finalize;

/// Defines the schedules to be run for the [`WalkingEngineSchedule`] schedule, including
/// their order.
#[derive(Resource, Debug)]
pub struct WalkingEngineScheduleOrder {
    /// The labels to run for [`WalkingEngineSchedule`] (in the order they will be run).
    pub labels: Vec<InternedScheduleLabel>,
}

impl Default for WalkingEngineScheduleOrder {
    fn default() -> Self {
        Self {
            labels: vec![
                StepPlanning.intern(),
                GaitGeneration.intern(),
                Balancing.intern(),
                Finalize.intern(),
            ],
        }
    }
}

/// The schedule that contains all systems related to the walking engine.
///
/// This needs to be a separate schedule in order to ensure correct ordering and that state updates
/// happen as soon as they can.
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
struct WalkingEngine;

impl WalkingEngine {
    fn run_walkinge_engine(world: &mut World) {
        world.resource_scope(|world, order: Mut<WalkingEngineScheduleOrder>| {
            for &label in &order.labels {
                let _ = world.try_run_schedule(label);
            }
        });
    }
}

/// Plugin that sets up the system sets that define the walking engine.
pub(super) struct WalkingEngineSchedulePlugin;

impl Plugin for WalkingEngineSchedulePlugin {
    fn build(&self, app: &mut App) {
        let mut walking_engine_schedule = Schedule::new(WalkingEngine);

        // simple "facilitator" schedules benefit from simpler single threaded scheduling
        walking_engine_schedule.set_executor_kind(ExecutorKind::SingleThreaded);
        walking_engine_schedule.set_build_settings(ScheduleBuildSettings {
            ambiguity_detection: LogLevel::Warn,
            ..default()
        });

        app.add_schedule(walking_engine_schedule)
            .init_resource::<WalkingEngineScheduleOrder>()
            .add_systems(WalkingEngine, WalkingEngine::run_walkinge_engine);

        app.world_mut()
            .resource_mut::<MainScheduleOrder>()
            .insert_after(PostUpdate, WalkingEngine);

        app.init_state::<Gait>();
        app.configure_sets(PostStartup, MotionSet::order())
            .configure_sets(Update, MotionSet::order())
            .configure_sets(PostUpdate, MotionSet::order())
            .configure_sets(OnEnter(Gait::Sitting), MotionSet::order())
            .configure_sets(OnEnter(Gait::Standing), MotionSet::order())
            .configure_sets(OnEnter(Gait::Walking), MotionSet::order());

        app.add_systems(PostStartup, setup_motion_state.in_set(MotionSet::Finalize));
    }
}

/// System that sets the initial [`MotionState`] depending on the initial hip height.
fn setup_motion_state(
    mut commands: Commands,
    config: Res<WalkingEngineConfig>,
    kinematics: Res<Kinematics>,
) {
    let start = FootPositions::from_kinematics(Side::Left, &kinematics, TORSO_OFFSET);

    let hip_height = kinematics.left_hip_height();
    let planned = PlannedStep {
        step: Step::default(),
        start,
        target: start,
        duration: Duration::from_millis(250),
        swing_foot_height: 0.,
        swing_foot: Side::Left,
    };

    if hip_height >= config.max_sitting_hip_height {
        commands.insert_resource(StepManager::init(Gait::Standing, planned));
    } else {
        commands.insert_resource(StepManager::init(Gait::Sitting, planned));
    }
}
