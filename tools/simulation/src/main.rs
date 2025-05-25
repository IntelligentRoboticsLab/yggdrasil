use std::time::Instant;

use bevy::app::{AppLabel, MainSchedulePlugin};
use bevy::ecs::event::EventRegistry;
use bevy::state::app::StatesPlugin;
use bevy::{prelude::*, render::camera::Viewport, window::PrimaryWindow};
use bevy_egui::egui::{Direction, Layout, RichText, Ui};
use bevy_egui::{EguiContexts, EguiPlugin, egui};
use bifrost::communication::{
    CompetitionPhase, CompetitionType, GameControllerMessage, GamePhase, GameState, Half, Penalty,
    RobotInfo, SetPlay, TeamColor, TeamInfo,
};
use filter::{CovarianceMatrix, UnscentedKalmanFilter};
use nalgebra::{Isometry2, Point2, Translation2, UnitComplex, Vector2};
use tasks::TaskPlugin;
use yggdrasil::behavior::behaviors::Stand;
use yggdrasil::behavior::engine::CommandsBehaviorExt;
use yggdrasil::behavior::primary_state::PrimaryState;
use yggdrasil::core::audio::whistle_detection::Whistle;
use yggdrasil::core::config::layout::LayoutConfig;
use yggdrasil::core::config::layout::RobotPosition;
use yggdrasil::core::config::showtime::{self, PlayerConfig};
use yggdrasil::core::{config, control, debug};
use yggdrasil::localization::RobotPose;
use yggdrasil::localization::odometry::{self, Odometry};
use yggdrasil::motion::walking_engine::step_context::StepContext;
use yggdrasil::nao::Cycle;
use yggdrasil::prelude::Config;

use yggdrasil::sensor::orientation::update_orientation;
use yggdrasil::vision::ball_detection::ball_tracker::{BallPosition, BallTracker};
use yggdrasil::vision::referee::communication::ReceivedRefereePose;
use yggdrasil::vision::referee::recognize::RefereePoseRecognized;
use yggdrasil::{
    behavior, game_controller, kinematics, localization, motion, nao, schedule, sensor,
};

use bevy::ecs::schedule::ScheduleLabel;

// Constants for field dimensions
const FIELD_WIDTH_METERS: f32 = 10.4;
const FIELD_HEIGHT_METERS: f32 = 7.4;
// Remove fixed visual dimensions since we'll calculate them dynamically
const PIXELS_PER_METER: f32 = 100.0; // Base scale factor, will be adjusted dynamically
// Robot size in meters
const ROBOT_SIZE_METERS: f32 = 0.5; // 50cm x 50cm robot

// Scale factors to convert between meters and pixels - will be updated dynamically
#[derive(Resource)]
struct FieldScale {
    pixels_per_meter: f32,
}

#[derive(Component)]
struct PlayerNumber;

#[derive(Resource)]
pub struct Simulation {
    state: GameState,
}

impl Default for Simulation {
    fn default() -> Self {
        Self {
            state: GameState::Initial,
        }
    }
}

fn main() {
    let mut app = App::new();

    app.insert_resource(ClearColor(Color::srgb(0.25, 0.25, 0.25)))
        .insert_resource(FieldScale {
            pixels_per_meter: PIXELS_PER_METER,
        })
        .init_resource::<Simulation>()
        .add_plugins((
            DefaultPlugins,
            EguiPlugin {
                enable_multipass_for_primary_context: false,
            },
        ))
        .add_systems(Startup, setup_system)
        .add_systems(
            Update,
            (
                ui_main,
                update_robot_positions,
                update_field_scale,
                update_position_markers,
            ),
        );

    app.insert_sub_app(Robot1, create_full_robot(1));
    app.insert_sub_app(Robot2, create_full_robot(2));
    app.insert_sub_app(Robot3, create_full_robot(3));
    app.insert_sub_app(Robot4, create_full_robot(4));
    app.insert_sub_app(Robot5, create_full_robot(5));

    app.run();
}

macro_rules! define_robot_labels {
    ($($name:ident),*) => {
        $(
            #[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, AppLabel)]
            struct $name;
        )*
    };
}

define_robot_labels!(Robot1, Robot2, Robot3, Robot4, Robot5);

#[derive(Resource)]
struct PlayNum(u8);

fn create_full_robot(player_number: u8) -> SubApp {
    let mut sub_app = SubApp::new();

    sub_app.init_resource::<AppTypeRegistry>();
    sub_app.init_resource::<EventRegistry>();
    sub_app.insert_resource(PlayNum(player_number));

    let ball_tracker = BallTracker {
        position_kf: UnscentedKalmanFilter::<2, 5, BallPosition>::new(
            BallPosition(Point2::new(0.0, 0.0)),
            CovarianceMatrix::from_diagonal_element(0.001), // variance = std^2, and we don't know where the ball is
        ),
        // prediction is done each cycle, this is roughly 1.7cm of std per cycle or 1.3 meters per second
        prediction_noise: CovarianceMatrix::from_diagonal_element(0.001),
        sensor_noise: CovarianceMatrix::from_diagonal_element(0.001),
        cycle: Cycle::default(),
        timestamp: Instant::now(),
        stationary_variance_threshold: 0.001, // variance = std^2
    };

    sub_app
        .add_plugins((MinimalPlugins, MainSchedulePlugin, StatesPlugin))
        .add_plugins((
            schedule::NaoSchedulePlugin,
            game_controller::GameControllerPlugin,
            nao::SimulationNaoPlugins,
            TaskPlugin,
            ml::MlPlugin,
            config::ConfigPlugin,
            debug::DebugPlugin,
            control::ControlPlugin,
            localization::LocalizationPlugin,
            sensor::SensorPlugins,
            behavior::BehaviorPlugins,
            //TODO: Implement communication::CommunicationPlugins,
            kinematics::KinematicsPlugin,
            motion::MotionPlugins,
            // Removed vision::VisionPlugins,
        ))
        .insert_resource(ball_tracker)
        .insert_resource(Whistle::default())
        .insert_resource(initial_gamecontroller())
        .add_event::<RefereePoseRecognized>()
        .add_event::<ReceivedRefereePose>()
        .add_systems(PostStartup, (setup_robot, update_orientation))
        .add_systems(
            PreUpdate,
            (update_simulated_odometry.after(odometry::update_odometry)),
        )
        .add_systems(
            PreStartup,
            set_player_number.after(showtime::configure_showtime),
        );

    fn setup_robot(mut commands: Commands) {
        commands.insert_resource(PrimaryState::Initial);
        commands.set_behavior(Stand);
    }

    fn set_player_number(mut commands: Commands, play_num: Res<PlayNum>) {
        commands.insert_resource(PlayerConfig {
            player_number: play_num.0,
            team_number: 8,
        });
    }

    fn update_simulated_odometry(mut commands: Commands, step_context: Res<StepContext>) {
        let step = step_context.requested_step;

        let translation = Vector2::new(step.forward, step.left);
        let rotation = UnitComplex::from_angle(step.turn / 30.0);

        let mut new_odom = Odometry::new();
        new_odom.offset_to_last =
            Isometry2::from_parts(Translation2::from(translation * 0.2), rotation);
        commands.insert_resource(new_odom);
    }

    sub_app.set_extract(|main_world, sub_world| {
        let simulation = main_world.resource_mut::<Simulation>();
        sub_world.resource_mut::<GameControllerMessage>().state = simulation.state;

        let robot_pose = sub_world.resource::<RobotPose>();

        let player_config = sub_world.resource::<PlayerConfig>();

        for mut robot in main_world.query::<&mut Robot>().iter_mut(main_world) {
            if robot.player_number == player_config.player_number {
                robot.position = robot_pose.inner.translation.vector.into();
                robot.rotation = robot_pose.inner.rotation.angle();
            }
        }
    });

    sub_app.update_schedule = Some(Main.intern());
    sub_app.run_default_schedule();

    sub_app
}

fn initial_gamecontroller() -> GameControllerMessage {
    GameControllerMessage {
        header: Default::default(),
        version: Default::default(),
        packet_number: Default::default(),
        players_per_team: Default::default(),
        competition_phase: CompetitionPhase::RoundRobin,
        competition_type: CompetitionType::Normal,
        game_phase: GamePhase::Normal,
        state: GameState::Initial,
        set_play: SetPlay::None,
        first_half: Half::First,
        kicking_team: Default::default(),
        secs_remaining: Default::default(),
        secondary_time: Default::default(),
        teams: [TeamInfo {
            team_number: 8,
            field_player_colour: TeamColor::Red,
            goalkeeper_colour: TeamColor::Black,
            goalkeeper: 1,
            score: 0,
            penalty_shot: 0,
            single_shots: 0,
            message_budget: 1200,
            players: [RobotInfo {
                penalty: Penalty::None,
                secs_till_unpenalised: 0,
            }; 20],
        }; 2],
    }
}

#[derive(Component)]
struct Field;

#[derive(Component)]
struct PositionCircle;

// Real-world robot position in meters
#[derive(Component)]
struct Robot {
    player_number: u8,
    // Position in meters from field center
    position: Vec2,
    // Rotation in radians
    rotation: f32,
}

impl Robot {
    fn new(player_number: u8, x_meters: f32, y_meters: f32, rotation_degrees: f32) -> Self {
        Self {
            player_number,
            position: Vec2::new(x_meters, y_meters),
            rotation: rotation_degrees.to_radians(),
        }
    }

    fn to_screen_position(&self, field_scale: &FieldScale) -> Vec3 {
        Vec3::new(
            self.position.x * field_scale.pixels_per_meter,
            self.position.y * field_scale.pixels_per_meter,
            1.0,
        )
    }
}

// System to update visual positions based on Robot components
fn update_robot_positions(
    field_scale: Res<FieldScale>,
    mut query: Query<(&Robot, &mut Transform)>,
    mut sprite_query: Query<&mut Sprite, With<Robot>>,
) {
    // Update positions
    for (robot, mut transform) in query.iter_mut() {
        transform.translation = robot.to_screen_position(&field_scale);
        transform.rotation = Quat::from_rotation_z(robot.rotation);
    }

    // Update sprite sizes
    for mut sprite in sprite_query.iter_mut() {
        sprite.custom_size = Some(Vec2::splat(
            ROBOT_SIZE_METERS * field_scale.pixels_per_meter,
        ));
    }
}

fn ui_main(
    mut contexts: EguiContexts,
    mut camera: Single<&mut Camera>,
    mut simulation: ResMut<Simulation>,
    window: Single<&mut Window, With<PrimaryWindow>>,
) {
    let ctx = contexts.ctx_mut();

    let top_height = egui::TopBottomPanel::top("top_panel")
        .resizable(true)
        .min_height(100.0)
        .show(ctx, |ui| {
            ui_panel_top(&mut simulation, ui);
        })
        .response
        .rect
        .height();

    // Set initial size to 25% of window width, max 50%
    let right = egui::SidePanel::right("right_panel")
        .default_width(window.width() * 0.25)
        .max_width(window.width() * 0.5)
        .resizable(true)
        .show(ctx, |ui| {
            ui.label("Right resizeable panel");
            ui.allocate_rect(ui.available_rect_before_wrap(), egui::Sense::hover());
        })
        .response
        .rect
        .width();

    // Set initial size to 25% of window height, max 33%
    let bottom = egui::TopBottomPanel::bottom("bottom_panel")
        .default_height(window.height() * 0.25)
        .max_height(window.height() * 0.33)
        .resizable(true)
        .show(ctx, |ui| {
            ui.label("Bottom resizeable panel");
            ui.allocate_rect(ui.available_rect_before_wrap(), egui::Sense::hover());
        })
        .response
        .rect
        .height();

    // Scale from logical units to physical units
    let right_scaled = right * window.scale_factor();
    let bottom_scaled = bottom * window.scale_factor();
    let top_scaled = top_height * window.scale_factor();

    let pos = UVec2::new(0, top_scaled as u32);
    let size = UVec2::new(window.physical_width(), window.physical_height())
        - UVec2::new(
            right_scaled as u32,
            bottom_scaled as u32 + top_scaled as u32,
        );

    camera.viewport = Some(Viewport {
        physical_position: pos,
        physical_size: size,
        ..default()
    });
}

fn ui_panel_top(simulation: &mut Simulation, ui: &mut Ui) {
    let layout = Layout {
        main_dir: Direction::LeftToRight,
        main_wrap: false,
        cross_justify: false,
        ..Default::default()
    };
    ui.with_layout(layout, |ui| {
        let mut button_size = ui.available_size();
        button_size.x /= 5.0;

        let layout = Layout::centered_and_justified(Direction::TopDown);
        let game_states = [
            GameState::Initial,
            GameState::Ready,
            GameState::Set,
            GameState::Playing,
            GameState::Finished,
        ];
        for state in &game_states {
            ui.allocate_ui_with_layout(button_size, layout, |ui| {
                ui.selectable_value(
                    &mut simulation.state,
                    *state,
                    RichText::new(format!("{:?}", state))
                        .size(40.0)
                        .text_style(egui::TextStyle::Heading),
                );
            });
        }
    });
}

fn update_field_scale(
    window: Query<&Window, With<PrimaryWindow>>,
    mut field_scale: ResMut<FieldScale>,
    mut field_query: Query<&mut Sprite, With<Field>>,
    camera: Query<&Camera>,
) {
    let window = window.single().expect("Simulation did not find a window!");
    let camera = camera.single().expect("Simulation did not find a camera!");

    if let Some(viewport) = &camera.viewport {
        let available_width = viewport.physical_size.x as f32 / window.scale_factor();
        let available_height = viewport.physical_size.y as f32 / window.scale_factor();

        // Calculate scale factors to fit width and height
        let scale_x = available_width / FIELD_WIDTH_METERS;
        let scale_y = available_height / FIELD_HEIGHT_METERS;

        // Use the smaller scale to maintain aspect ratio
        let new_scale = scale_x.min(scale_y);
        field_scale.pixels_per_meter = new_scale;

        // Update field sprite size
        if let Ok(mut sprite) = field_query.single_mut() {
            sprite.custom_size = Some(Vec2::new(
                FIELD_WIDTH_METERS * new_scale,
                FIELD_HEIGHT_METERS * new_scale,
            ));
        }
    }
}

// Add new system to update position markers
fn update_position_markers(
    field_scale: Res<FieldScale>,
    mut circle_query: Query<&mut Transform, (With<PositionCircle>, Without<PlayerNumber>)>,
    mut text_query: Query<
        (&mut Transform, &ChildOf),
        (With<PlayerNumber>, Without<PositionCircle>),
    >,
    robot_transforms: Query<
        &Transform,
        (With<Robot>, Without<PositionCircle>, Without<PlayerNumber>),
    >,
) {
    // Update circle transforms (they will inherit position from parent)
    let circle_scale = ROBOT_SIZE_METERS * 0.30 * field_scale.pixels_per_meter;
    for mut transform in circle_query.iter_mut() {
        transform.scale = Vec3::splat(circle_scale);
    }

    // Update text transforms - counter-rotate against parent
    for (mut transform, child) in text_query.iter_mut() {
        // Get the parent's rotation and apply the inverse to keep text upright
        if let Ok(parent_transform) = robot_transforms.get(child.parent()) {
            transform.rotation = parent_transform.rotation.inverse();
        }
    }
}

#[allow(dead_code)]
fn on_robot_drag(
    drag: Trigger<Pointer<Drag>>,
    mut robots: Query<(&mut Robot, &mut Transform)>,
    field_scale: Res<FieldScale>,
) {
    // First try direct access (in case the Robot component is on the dragged entity)
    if let Ok((mut robot, mut transform)) = robots.get_mut(drag.target.entity()) {
        let world_delta = Vec2::new(
            drag.delta.x / field_scale.pixels_per_meter,
            -drag.delta.y / field_scale.pixels_per_meter,
        );
        robot.position += world_delta;
        transform.translation = robot.to_screen_position(&field_scale);
    }
}

fn draw_robot(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<ColorMaterial>>,
    config: &RobotPosition,
    robot_texture: &Handle<Image>,
    text_font: &TextFont,
    text_justification: JustifyText,
    field_scale: &FieldScale,
) {
    let color = Color::srgba(0.2, 1.0, 0.2, 0.5);

    // Create the robot with proper initial position
    let robot = Robot::new(
        config.player_number as u8,
        config.isometry.translation.vector.x,
        config.isometry.translation.vector.y,
        config.isometry.rotation.angle().to_degrees(),
    );

    // Calculate initial screen position
    let screen_pos = robot.to_screen_position(field_scale);

    // Create robot entity with correct initial transform
    let robot_entity = commands
        .spawn((
            Transform::from_translation(screen_pos),
            robot,
            Visibility::default(),
            InheritedVisibility::default(),
            Sprite {
                image: robot_texture.clone(),
                custom_size: Some(Vec2::splat(
                    ROBOT_SIZE_METERS * field_scale.pixels_per_meter,
                )),
                ..default()
            },
        ))
        .id();

    // Spawn the draggable circle
    commands
        .spawn((
            Mesh2d(meshes.add(Circle::new(1.0))),
            MeshMaterial2d(materials.add(ColorMaterial::from_color(color))),
            Transform::from_xyz(0.0, 0.0, 0.0),
            PositionCircle,
        ))
        // .observe(|over: Trigger<Pointer<Over>>| {
        //     println!("Over event triggered for circle: {}", over.entity());
        // })
        .set_parent(robot_entity);

    commands
        .spawn((
            Text2d::new(config.player_number.to_string()),
            text_font.clone(),
            TextLayout::new_with_justify(text_justification),
            Transform::from_xyz(0.0, 0.0, 0.2),
            TextColor(Color::srgb(0.2, 0.2, 0.2)),
            PlayerNumber,
        ))
        .set_parent(robot_entity);

    // Add drag observer directly to the robot entity
    // commands.entity(robot_entity).observe(on_robot_drag);
}

fn setup_system(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    field_scale: Res<FieldScale>,
) {
    let field_texture = asset_server.load("field_simple.png");
    let robot_texture = asset_server.load("nao.png");
    let font = asset_server.load("fonts/FiraSans-Bold.ttf");
    let layout_config = LayoutConfig::load("config/").expect("Failed to load layout config");

    let text_font = TextFont {
        font: font.clone(),
        font_size: 20.0,
        ..default()
    };
    let text_justification = JustifyText::Center;

    // Spawn the field
    commands.spawn((
        Sprite {
            image: field_texture.clone(),
            custom_size: Some(Vec2::new(
                FIELD_WIDTH_METERS * PIXELS_PER_METER,
                FIELD_HEIGHT_METERS * PIXELS_PER_METER,
            )),
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, 0.0),
        Field,
    ));

    // Spawn all robots in a single loop
    for i in 1..=layout_config.initial_positions.len() {
        // Draw initial position robot
        draw_robot(
            &mut commands,
            &mut meshes,
            &mut materials,
            &layout_config.initial_positions.player(i as u8),
            &robot_texture,
            &text_font,
            text_justification,
            &field_scale, // Pass field_scale here
        );
    }

    commands.spawn(Camera2d);
}
