#! [windows_subsystem = "windows"]
use bevy::{prelude::*, render::camera::Viewport, window::PrimaryWindow};
use bevy_egui::{egui, EguiContexts, EguiPlugin};

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::srgb(0.25, 0.25, 0.25)))
        .add_plugins(DefaultPlugins)
        .add_plugins(EguiPlugin)
        .add_systems(Startup, setup_system)
        .add_systems(Update, ui_example_system)
        .run();
}

// This function runs every frame. Therefore, updating the viewport after drawing the gui.
// With a resource which stores the dimensions of the panels, the update of the Viewport can
// be done in another system.
fn ui_example_system(
    mut contexts: EguiContexts,
    mut camera: Single<&mut Camera>,
    window: Single<&mut Window, With<PrimaryWindow>>,
) {
    // egui context
    let ctx = contexts.ctx_mut();

    let mut left = egui::SidePanel::left("left_panel")
        .resizable(true)
        .show(ctx, |ui| {
            ui.label("Left resizeable panel");
            ui.allocate_rect(ui.available_rect_before_wrap(), egui::Sense::hover());
        })
        .response
        .rect
        .width(); // height is ignored, as the panel has a hight of 100% of the screen

    let mut right = egui::SidePanel::right("right_panel")
        .resizable(true)
        .show(ctx, |ui| {
            ui.label("Right resizeable panel");
            ui.allocate_rect(ui.available_rect_before_wrap(), egui::Sense::hover());
        })
        .response
        .rect
        .width(); // height is ignored, as the panel has a height of 100% of the screen

    let mut top = egui::TopBottomPanel::top("top_panel")
        .resizable(true)
        .show(ctx, |ui| {
            ui.label("Top resizeable panel");
            ui.allocate_rect(ui.available_rect_before_wrap(), egui::Sense::hover());
        })
        .response
        .rect
        .height(); // width is ignored, as the panel has a width of 100% of the screen
    let mut bottom = egui::TopBottomPanel::bottom("bottom_panel")
        .resizable(true)
        .show(ctx, |ui| {
            ui.label("Bottom resizeable panel");
            ui.allocate_rect(ui.available_rect_before_wrap(), egui::Sense::hover());
        })
        .response
        .rect
        .height(); // width is ignored, as the panel has a width of 100% of the screen

    // Scale from logical units to physical units
    left *= window.scale_factor();
    right *= window.scale_factor();
    top *= window.scale_factor();
    bottom *= window.scale_factor();

    // -------------------------------------------------
    // |  left   |            top   ^^^^^^   |  right  |
    // |  panel  |           panel  height   |  panel  |
    // |         |                  vvvvvv   |         |
    // |         |---------------------------|         |
    // |         |                           |         |
    // |<-width->|        2D viewport        |<-width->|
    // |         |                           |         |
    // |         |---------------------------|         |
    // |         |          bottom   ^^^^^^  |         |
    // |         |          panel    height  |         |
    // |         |                   vvvvvv  |         |
    // -------------------------------------------------
    //
    // The upper left point of the viewport is the width of the left panel and the height of the
    // top panel
    //
    // The width of the 2D viewport the width of the top/bottom panel
    // Alternative the width can be calculated as follow:
    // size.x = window width - left panel width - right panel width
    //
    // The height of the 2d viewport is:
    // size.y = window height - top panel height - bottom panel height
    //
    // Therefore we use the alternative for the width, as we can callculate the Viewport as
    // following:

    let pos = UVec2::new(left as u32, top as u32);
    let size = UVec2::new(window.physical_width(), window.physical_height())
        - pos
        - UVec2::new(right as u32, bottom as u32);

    camera.viewport = Some(Viewport {
        physical_position: pos,
        physical_size: size,
        ..default()
    });
}

// Set up the example entities for the 2D scene. The only important thing is a 2D Camera which
// renders directly to the window.
fn setup_system(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    // Circle
    commands.spawn((
        Mesh2d(meshes.add(Circle::new(50.))),
        MeshMaterial2d(materials.add(ColorMaterial::from_color(Color::srgb(0.2, 0.1, 0.0)))),
        Transform::from_translation(Vec3::new(-150., 0., 0.)),
    ));

    // Rectangles
    commands.spawn((
        Mesh2d(meshes.add(Rectangle::new(50., 100.))),
        MeshMaterial2d(materials.add(ColorMaterial::from_color(Color::srgb(0.5, 0.4, 0.3)))),
        Transform::from_translation(Vec3::new(-50., 0., 0.)),
    ));

    commands.spawn((
        Mesh2d(meshes.add(Rectangle::new(50., 100.))),
        MeshMaterial2d(materials.add(ColorMaterial::from_color(Color::srgb(0.5, 0.4, 0.3)))),
        Transform::from_translation(Vec3::new(50., 0., 0.)),
    ));

    // Hexagon
    commands.spawn((
        Mesh2d(meshes.add(RegularPolygon::new(50., 6))),
        MeshMaterial2d(materials.add(ColorMaterial::from_color(Color::srgb(0.8, 0.7, 0.6)))),
        Transform::from_translation(Vec3::new(150., 0., 0.)),
    ));

    commands.spawn(Camera2d);
}

/*
// // .---------------------------------------------------------------------------.
// // |                                                                           |
// // |                                                                           |
// // |        <----------------------------720--------------------------->         |
// // |      .------------------------------------------------------------.       |
// // |    ^ |                              |                             |       |
// // |    | | <--G-->                      |                             |       |
// // |    | |----------.                   |                  .----------|       |
// // |    | |          | ^                 |                  |          |       |
// // |    | |<E>       | |                 |                  |          |       |
// // |    | |---.      | |                 |                  |      .---|       |
// // |    2 |   | ^    | |               -----                |      |   |       |
// // |    1 |   | |    | |              /  |  \               |      |   |       |
// // |    5 |   | F 0  | H             |<--J-->|              |  0<--I-->|       |
// // |    | |   | |    | |              \  |  /               |      |   |       |
// // |    | |   | v    | |               -----                |      |   |       |
// // |    | |---.      | |                 |                  |      .---|       |
// // |    | |          | |                 |                  |          |       |
// // |    | |          | v                 |                  |          |       |
// // |    | |----------.                   |                  .----------|       |
// // |    | |                              |                             |       |
// // |    v |                              |                             |<--K-->|
// // |      .------------------------------------------------------------.       |
// // |                                                                 ^         |
// // |                                                                 K         |
// // |                                                                 v         |
// // .---------------------------------------------------------------------------.
// 2811x2000
// 702.75 x 500
// 10.4x7.4
// 270x270

use bifrost::communication::{
    CompetitionPhase, CompetitionType, GameControllerMessage, GamePhase, GameState, Half, Penalty,
    RobotInfo, SetPlay, TeamColor, TeamInfo,
};
use egui::{emath::RectTransform, Pos2, Rect};
use egui::{
    Color32, Direction, Image, Layout, Painter, Response, RichText, Sense, Stroke, Ui, Vec2,
};
use nalgebra::{Isometry2, Point2, Vector2};
use std::time::Duration;
use yggdrasil::behavior::behaviors::ObserveBehaviorConfig;
use yggdrasil::behavior::engine::{BehaviorKind, Context};
use yggdrasil::behavior::primary_state::{next_primary_state, PrimaryStateConfig};
use yggdrasil::behavior::BehaviorConfig;
use yggdrasil::core::config::showtime::PlayerConfig;
use yggdrasil::core::config::yggdrasil::YggdrasilConfig;
use yggdrasil::core::debug::DebugContext;
use yggdrasil::core::whistle::WhistleState;
use yggdrasil::game_controller::GameControllerConfig;
use yggdrasil::localization::{next_robot_pose, RobotPose};
use yggdrasil::motion::odometry::{Odometry, OdometryConfig};
use yggdrasil::motion::step_planner::StepPlanner;
use yggdrasil::motion::walk::engine::WalkRequest;
use yggdrasil::prelude::Config;
use yggdrasil::sensor::orientation::OrientationFilterConfig;
use yggdrasil::sensor::{ButtonConfig, FsrConfig, SensorConfig};
use yggdrasil::vision::camera::{CameraConfig, CameraSettings};
use yggdrasil::vision::field_marks::FieldMarksConfig;
use yggdrasil::vision::VisionConfig;
use yggdrasil::{
    behavior::{engine::Control, primary_state::PrimaryState, BehaviorEngine},
    core::config::layout::LayoutConfig,
    motion::walk::engine::WalkingEngine,
};

#[derive(Default)]
struct OccupiedScreenSpace {
    right: f32,
    bottom: f32,
}

fn main() -> eframe::Result<()> {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_fullscreen(true),
        ..Default::default()
    };
    eframe::run_native(
        "Behavior Simulation",
        options,
        Box::new(|cc| {
            // Provide image support:
            egui_extras::install_image_loaders(&cc.egui_ctx);

            Ok(Box::<Simulation>::default())
        }),
    )
}

const NUMBER_OF_PLAYERS: usize = 5;
const FRAMES_PER_SECOND: u64 = 120;

struct Simulation {
    occupied_screen_space: OccupiedScreenSpace,
    gamecontrollermessage: GameControllerMessage,
    penalties: [bool; NUMBER_OF_PLAYERS],
    game_state: GameState,
    robots: Vec<Robot>,
    layout_config: LayoutConfig,
    global_ball: Option<Point2<f32>>,
}

impl Default for Simulation {
    fn default() -> Self {
        let layout_config = LayoutConfig::load("../../deploy/config/").unwrap();

        let gamecontrollermessage = GameControllerMessage {
            competition_phase: CompetitionPhase::PlayOff,

            competition_type: CompetitionType::Normal,

            game_phase: GamePhase::Normal,

            state: GameState::Initial,

            set_play: SetPlay::None,

            first_half: Half::First,

            teams: [TeamInfo {
                field_player_colour: TeamColor::Blue,
                goalkeeper_colour: TeamColor::Blue,
                players: [RobotInfo {
                    penalty: Penalty::None,
                    secs_till_unpenalised: 0,
                }; 20],
                team_number: 8,
                goalkeeper: Default::default(),
                score: Default::default(),
                penalty_shot: Default::default(),
                single_shots: Default::default(),
                message_budget: Default::default(),
            }; 2],
            header: Default::default(),
            version: Default::default(),
            packet_number: Default::default(),
            players_per_team: Default::default(),
            kicking_team: Default::default(),
            secs_remaining: Default::default(),
            secondary_time: Default::default(),
        };

        let robots = (0..NUMBER_OF_PLAYERS)
            .map(|i| {
                Robot::new(
                    PlayerConfig {
                        player_number: (i + 1) as u8,
                        team_number: 8,
                    },
                    layout_config
                        .initial_positions
                        .player((i + 1) as u8)
                        .isometry,
                )
            })
            .collect();

        Self {
            occupied_screen_space: OccupiedScreenSpace::default(),
            gamecontrollermessage,
            penalties: [false; NUMBER_OF_PLAYERS],
            game_state: GameState::Initial,
            robots,
            layout_config,
            global_ball: Some(Point2::new(0.0, 0.0)),
        }
    }
}

impl Simulation {
    fn absolute_to_simulation(image_response: &Response, point: Point2<f32>) -> Pos2 {
        let to_screen = RectTransform::from_to(
            Rect::from_min_size(Pos2::ZERO, image_response.rect.size()),
            image_response.rect,
        );

        let field_scaler = image_response.rect.size().y / 7.4;
        let field_center = image_response.rect.size().to_pos2() / 2.0;

        let pos =
            to_screen.transform_pos(field_center + field_scaler * Vec2::new(point.x, -point.y));
        Pos2::new(pos.x, pos.y)
    }

    fn simulation_to_absolute(image_response: &Response, pos: Pos2) -> Point2<f32> {
        let from_screen = RectTransform::from_to(
            Rect::from_min_size(Pos2::ZERO, image_response.rect.size()),
            image_response.rect,
        )
        .inverse();

        let field_scaler = image_response.rect.size().y / 7.4;
        let field_center = image_response.rect.size().to_pos2() / 2.0;

        let pos = (from_screen.transform_pos(pos) - field_center) / field_scaler;

        Point2::new(pos.x, -pos.y)
    }

    fn check_ball_collisions(&mut self) {
        let ball = self.global_ball.unwrap();

        for robot in self.robots.iter() {
            let robot_pos = robot.pose.world_position();
            let robot_radius = 0.1; // Robot radius
            let ball_radius = 0.05; // Ball radius

            let distance = (robot_pos - ball).norm();
            if distance < robot_radius + ball_radius {
                // Move the ball to the edge of the robot
                let direction = (ball - robot_pos).normalize();
                let new_ball_pos = robot_pos + direction * (robot_radius + ball_radius);
                self.global_ball = Some(new_ball_pos);
            }
        }
    }

    fn draw_ball(&self, painter: &Painter, image_response: &Response) {
        if let Some(ball) = self.global_ball {
            painter.circle_filled(
                Simulation::absolute_to_simulation(image_response, ball),
                12.0f32,
                Color32::BLUE,
            );
        }
    }

    fn update_global_ball(&mut self, response: &Response) {
        if let Some(pointer_pos) = response.interact_pointer_pos() {
            self.global_ball = Some(Simulation::simulation_to_absolute(response, pointer_pos));
        }
        self.check_ball_collisions();
    }

    fn ui_panel_top(&mut self, ui: &mut Ui) {
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
                        &mut self.game_state,
                        *state,
                        RichText::new(format!("{:?}", state))
                            .size(40.0)
                            .text_style(egui::TextStyle::Heading),
                    );
                });
            }
        });
    }

    fn ui_panel_right(&mut self, ui: &mut Ui) {
        ui.label(RichText::new("Penalties").heading());
        ui.columns(NUMBER_OF_PLAYERS, |columns| {
            for (i, column) in columns.iter_mut().enumerate() {
                column.label(RichText::new(format!("{:?}", i + 1)));
                column.checkbox(&mut self.penalties[i], "");
            }
        });
    }

    fn ui_panel_bottom(&mut self, ui: &mut Ui) {
        ui.columns(NUMBER_OF_PLAYERS, |columns| {
            self.robots.iter().enumerate().for_each(|(i, robot)| {
                columns[i].label(
                    RichText::new(format!("Robot {:?}", robot.player_config.player_number))
                        .strong(),
                );
                columns[i].label(format!("{:?}", robot.primary_state));
                columns[i].label(format!("{:?}", robot.engine.behavior));
                columns[i].label(format!("{:?}", robot.engine.role));
            });
        });
    }

    fn update_panel_top(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("top_panel")
            .resizable(true)
            .min_height(100.0)
            .show(ctx, |ui| {
                self.ui_panel_top(ui);
            });
    }

    fn update_panel_right(&mut self, ctx: &egui::Context) {
        self.occupied_screen_space.right = egui::SidePanel::right("right_panel")
            .resizable(true)
            .min_width(500.0)
            .show(ctx, |ui| {
                self.ui_panel_right(ui);
            })
            .response
            .rect
            .width();
    }

    fn update_panel_bottom(&mut self, ctx: &egui::Context) {
        self.occupied_screen_space.bottom = egui::TopBottomPanel::bottom("bottom_panel")
            .resizable(true)
            .min_height(250.0)
            .show(ctx, |ui| {
                self.ui_panel_bottom(ui);
            })
            .response
            .rect
            .height();
    }

    fn update_panel_center(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let img_source = egui::include_image!("./assets/field_simple.png");

            let image_response = ui.add(
                Image::new(img_source)
                    .sense(Sense::click_and_drag())
                    .maintain_aspect_ratio(true)
                    .max_width(ui.available_width() - self.occupied_screen_space.right)
                    .max_height(ui.available_height() - self.occupied_screen_space.bottom)
                    .rounding(10.0),
            );

            let painter = ui.painter_at(image_response.rect);

            self.gamecontrollermessage.state = self.game_state;

            for (i, penalty) in self.penalties.iter().enumerate() {
                if *penalty {
                    self.gamecontrollermessage.teams[0].players[i].penalty = Penalty::Manual;
                } else {
                    self.gamecontrollermessage.teams[0].players[i].penalty = Penalty::None;
                }
            }

            for robot in self.robots.iter_mut() {
                robot.update(
                    &self.gamecontrollermessage,
                    &self.global_ball,
                    &self.layout_config,
                );
                robot.draw(ui, &painter, &image_response, &self.global_ball);
            }
            self.update_global_ball(&image_response);
            self.draw_ball(&painter, &image_response);
        });
    }
}

impl eframe::App for Simulation {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.request_repaint_after(Duration::from_millis(1000 / FRAMES_PER_SECOND));
        self.update_panel_top(ctx);
        self.update_panel_center(ctx);
        self.update_panel_right(ctx);
        self.update_panel_bottom(ctx);
    }
}

struct Robot {
    player_config: PlayerConfig,
    primary_state: PrimaryState,
    pose: RobotPose,
    engine: BehaviorEngine,
    walking_engine: WalkingEngine,
    step_planner: StepPlanner,
    sees_ball: bool,
}

impl Robot {
    fn new(player_config: PlayerConfig, isometry: Isometry2<f32>) -> Self {
        Self {
            walking_engine: WalkingEngine::default(),
            step_planner: StepPlanner::default(),
            engine: BehaviorEngine::default(),
            primary_state: PrimaryState::Initial,
            pose: RobotPose { inner: isometry },
            sees_ball: false,
            player_config,
        }
    }

    fn update(
        &mut self,
        gamecontrollermessage: &GameControllerMessage,
        ball: &Option<Point2<f32>>,
        layout_config: &LayoutConfig,
    ) {
        self.primary_state = next_primary_state(
            &self.primary_state,
            &Some(gamecontrollermessage.clone()),
            &Default::default(),
            &Default::default(),
            &self.player_config,
            // &WhistleState::default(),
        );
        let mut control = Control {
            nao_manager: &mut Default::default(),
            keyframe_executor: &mut Default::default(),
            step_planner: &mut self.step_planner,
            walking_engine: &mut self.walking_engine,
            debug_context: &mut DebugContext::init("kaas", std::net::IpAddr::from([0, 0, 0, 0]))
                .unwrap(),
        };
        let (yggdrasil_config, behavior_config, game_controller_config) = create_default_configs();
        let context = Context {
            robot_info: &Default::default(),
            head_buttons: &Default::default(),
            chest_button: &Default::default(),
            contacts: &Default::default(),
            behavior_config: &behavior_config,
            // game_controller_config: &game_controller_config,
            yggdrasil_config: &yggdrasil_config,

            fall_state: &Default::default(),
            primary_state: &self.primary_state,
            player_config: &self.player_config,
            layout_config,
            game_controller_message: Some(gamecontrollermessage),
            pose: &self.pose,
            current_behavior: BehaviorKind::Stand(Default::default()),
            ball_position: if self.sees_ball { ball } else { &None },
        };

        self.engine.step(context, &mut control);

        self.update_ball(ball);
        self.walk(0.1, layout_config, gamecontrollermessage);
    }

    fn walk(
        &mut self,
        walk_scalar: f32,
        layout_config: &LayoutConfig,
        gamecontrollermessage: &GameControllerMessage,
    ) {
        let step = match self.walking_engine.request {
            WalkRequest::Walk(step) => Some(step),
            _ => None,
        };
        let mut odometry = Odometry::default();
        odometry.offset_to_last = if let Some(step) = step {
            Isometry2::new(
                Vector2::new(step.forward, step.left) * walk_scalar,
                step.turn / FRAMES_PER_SECOND as f32,
            )
        } else {
            Isometry2::identity()
        };

        self.pose = next_robot_pose(
            &self.pose,
            &odometry,
            &self.primary_state,
            layout_config,
            &Some(gamecontrollermessage.clone()),
        );
    }

    fn update_ball(&mut self, ball: &Option<Point2<f32>>) {
        let Some(ball) = ball else {
            self.sees_ball = false;
            return;
        };

        let relative_ball = self.pose.world_to_robot(ball);
        let angle = self.pose.angle_to(&ball);

        self.sees_ball = relative_ball.coords.norm() < 3.0 && angle.abs() < 45.0f32.to_radians();
    }

    fn draw(
        &self,
        ui: &mut Ui,
        painter: &Painter,
        image_response: &Response,
        ball: &Option<Point2<f32>>,
    ) {
        let robot_rotation = self.pose.inner.rotation.inverse().angle();

        let robot_pos_screen =
            Simulation::absolute_to_simulation(image_response, self.pose.world_position());

        painter.circle_filled(robot_pos_screen, 13.0f32, Color32::RED);
        painter.text(
            robot_pos_screen,
            egui::Align2::CENTER_CENTER,
            format!("{}", self.player_config.player_number),
            egui::FontId {
                size: 20.0,
                family: egui::FontFamily::Proportional,
            },
            Color32::BLACK,
        );

        ui.put(
            Rect::from_center_size(robot_pos_screen, Vec2::splat(40.0)),
            Image::new(egui::include_image!("./assets/nao.png"))
                .max_width(40.0)
                .rotate(robot_rotation, Vec2::splat(0.5)),
        );

        let Some(ball) = ball else {
            return;
        };
        if self.sees_ball {
            painter.line_segment(
                [
                    robot_pos_screen,
                    Simulation::absolute_to_simulation(image_response, *ball),
                ],
                Stroke::new(2.0, Color32::GREEN),
            );
        }
    }
}

fn create_default_configs() -> (YggdrasilConfig, BehaviorConfig, GameControllerConfig) {
    (
        YggdrasilConfig {
            // camera: CameraConfig {
            //     top: CameraSettings {
            //         path: Default::default(),
            //         width: 0,
            //         height: 0,
            //         calibration: Default::default(),
            //         exposure_auto: Default::default(),
            //         flip_horizontally: Default::default(),
            //         flip_vertically: Default::default(),
            //         focus_auto: Default::default(),
            //         num_buffers: 0,
            //     },
            //     bottom: CameraSettings {
            //         path: Default::default(),
            //         width: 0,
            //         height: 0,
            //         calibration: Default::default(),
            //         exposure_auto: Default::default(),
            //         flip_horizontally: Default::default(),
            //         flip_vertically: Default::default(),
            //         focus_auto: Default::default(),
            //         num_buffers: 0,
            //     },
            // },
            filter: SensorConfig {
                button: ButtonConfig {
                    activation_threshold: 0.0,
                    held_duration_threshold: Default::default(),
                },
                fsr: FsrConfig {
                    ground_contact_threshold: 0.0,
                },
            },
            game_controller: GameControllerConfig {
                game_controller_return_delay: Default::default(),
                game_controller_timeout: Default::default(),
            },
            odometry: OdometryConfig {
                scale_factor: Default::default(),
            },
            orientation: OrientationFilterConfig {
                acceleration_threshold: 0.0,
                acceleration_weight: 0.0,
                fsr_threshold: 0.0,
                gyro_threshold: 0.0,
            },
            primary_state: PrimaryStateConfig {
                chest_blink_interval: Default::default(),
            },
            // vision: VisionConfig {
            //     field_marks: FieldMarksConfig {
            //         angle_tolerance: 0.0,
            //         confidence_threshold: 0.0,
            //         distance_threshold: 0.0,
            //         patch_scale: 0.0,
            //         time_budget: 0,
            //     },
            // },
        },
        BehaviorConfig {
            observe: ObserveBehaviorConfig {
                head_pitch_max: 0.0,
                head_rotation_speed: 0.0,
                head_yaw_max: 0.0,
            },
        },
        GameControllerConfig {
            game_controller_timeout: Default::default(),
            game_controller_return_delay: Default::default(),
        },
    )
}
*/
