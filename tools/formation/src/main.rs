#![windows_subsystem = "windows"]
use bevy::{prelude::*, render::camera::Viewport, window::PrimaryWindow};
use bevy_egui::{egui, EguiContexts, EguiPlugin};
use yggdrasil::core::config::formation::{FormationConfig, RobotPosition};
use yggdrasil::prelude::Config;

// Constants for field dimensions
const FIELD_WIDTH_METERS: f32 = 10.4;
const FIELD_HEIGHT_METERS: f32 = 7.4;
// Remove fixed visual dimensions since we'll calculate them dynamically
const PIXELS_PER_METER: f32 = 100.0; // Base scale factor, will be adjusted dynamically
                                     // Robot size in meters
const ROBOT_SIZE_METERS: f32 = 0.5; // 50cm x 50cm robot

// Components to distinguish position types
#[derive(Component)]
struct InitialPosition;

#[derive(Component)]
struct SetPosition;

// Scale factors to convert between meters and pixels - will be updated dynamically
#[derive(Resource)]
struct FieldScale {
    pixels_per_meter: f32,
}

#[derive(Component)]
struct PlayerNumber;

// Define an enum to distinguish position types
#[derive(Debug, Clone, Copy)]
enum PositionType {
    Initial,
    Set,
}

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::srgb(0.25, 0.25, 0.25)))
        .insert_resource(FieldScale {
            pixels_per_meter: PIXELS_PER_METER,
        })
        .add_plugins((DefaultPlugins, EguiPlugin, MeshPickingPlugin))
        .add_systems(Startup, setup_system)
        .add_systems(
            Update,
            (
                ui_main,
                update_robot_positions,
                update_field_scale,
                update_position_markers,
            ),
        )
        .run();
}

#[derive(Component)]
struct Field;

#[derive(Component)]
struct RobotMarker;

#[derive(Component)]
struct PositionCircle;

// Real-world robot position in meters
#[derive(Component)]
struct Robot {
    // Position in meters from field center
    position: Vec2,
    // Rotation in radians
    rotation: f32,
}

impl Robot {
    fn new(x_meters: f32, y_meters: f32, rotation_degrees: f32) -> Self {
        Self {
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
    mut query: Query<(&Robot, &mut Transform, &mut Sprite), With<RobotMarker>>,
) {
    for (robot, mut transform, mut sprite) in query.iter_mut() {
        transform.translation = robot.to_screen_position(&field_scale);
        transform.rotation = Quat::from_rotation_z(robot.rotation);
        // Update robot sprite size based on field scale
        sprite.custom_size = Some(Vec2::splat(
            ROBOT_SIZE_METERS * field_scale.pixels_per_meter,
        ));
    }
}

fn ui_main(
    mut contexts: EguiContexts,
    mut camera: Single<&mut Camera>,
    window: Single<&mut Window, With<PrimaryWindow>>,
) {
    let ctx = contexts.ctx_mut();

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

    let pos = UVec2::new(0, 0);
    let size = UVec2::new(window.physical_width(), window.physical_height())
        - UVec2::new(right_scaled as u32, bottom_scaled as u32);

    camera.viewport = Some(Viewport {
        physical_position: pos,
        physical_size: size,
        ..default()
    });
}

fn update_field_scale(
    window: Query<&Window, With<PrimaryWindow>>,
    mut field_scale: ResMut<FieldScale>,
    mut field_query: Query<&mut Sprite, With<Field>>,
    camera: Query<&Camera>,
) {
    let window = window.single();
    let camera = camera.single();

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
        if let Ok(mut sprite) = field_query.get_single_mut() {
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
    mut initial_query: Query<
        (&Robot, &mut Transform),
        (
            With<InitialPosition>,
            Without<SetPosition>,
            Without<PositionCircle>,
            Without<PlayerNumber>,
        ),
    >,
    mut set_query: Query<
        (&Robot, &mut Transform),
        (
            With<SetPosition>,
            Without<InitialPosition>,
            Without<PositionCircle>,
            Without<PlayerNumber>,
        ),
    >,
    mut circle_query: Query<
        &mut Transform,
        (
            With<PositionCircle>,
            Without<InitialPosition>,
            Without<SetPosition>,
            Without<PlayerNumber>,
        ),
    >,
    mut text_query: Query<
        (&mut Transform, &Parent),
        (
            With<PlayerNumber>,
            Without<InitialPosition>,
            Without<SetPosition>,
            Without<PositionCircle>,
        ),
    >,
) {
    // Update initial positions
    for (robot, mut transform) in initial_query.iter_mut() {
        transform.translation = robot.to_screen_position(&field_scale);
        transform.rotation = Quat::from_rotation_z(robot.rotation);
    }

    // Update set positions
    for (robot, mut transform) in set_query.iter_mut() {
        transform.translation = robot.to_screen_position(&field_scale);
        transform.rotation = Quat::from_rotation_z(robot.rotation);
    }

    // Update circle transforms (they will inherit position from parent)
    let circle_scale = ROBOT_SIZE_METERS * 0.4 * field_scale.pixels_per_meter;
    for mut transform in circle_query.iter_mut() {
        transform.scale = Vec3::splat(circle_scale);
    }

    // Update text transforms - counter-rotate against parent
    for (mut transform, parent) in text_query.iter_mut() {
        // Get the parent's rotation and apply the inverse to keep text upright
        if let Ok(parent_transform) = initial_query.get(parent.get()) {
            transform.rotation = parent_transform.1.rotation.inverse();
        } else if let Ok(parent_transform) = set_query.get(parent.get()) {
            transform.rotation = parent_transform.1.rotation.inverse();
        }
    }
}

fn on_robot_drag(
    drag: Trigger<Pointer<Drag>>,
    mut robots: Query<(&mut Robot, &mut Transform), With<RobotMarker>>,
    field_scale: Res<FieldScale>,
    parents: Query<&Parent>,
) {
    if let Ok(parent) = parents.get(drag.entity()) {
        if let Ok((mut robot, mut transform)) = robots.get_mut(parent.get()) {
            let world_delta = Vec2::new(
                drag.delta.x / field_scale.pixels_per_meter,
                -drag.delta.y / field_scale.pixels_per_meter,
            );
            robot.position += world_delta;
            transform.translation = robot.to_screen_position(&field_scale);
        }
    }
}

fn draw_robot_with_type(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<ColorMaterial>>,
    config: &RobotPosition,
    position_type: PositionType,
    robot_texture: &Handle<Image>,
    text_font: &TextFont,
    text_justification: JustifyText,
) {
    let color = match position_type {
        PositionType::Initial => Color::srgba(0.2, 1.0, 0.2, 0.5),
        PositionType::Set => Color::srgba(1.0, 1.0, 0.2, 0.5),
    };

    let robot_entity = commands
        .spawn((
            Transform::default(),
            Robot::new(
                config.isometry.translation.vector.x,
                config.isometry.translation.vector.y,
                config.isometry.rotation.angle().to_degrees(),
            ),
            RobotMarker,
            Visibility::default(),
            InheritedVisibility::default(),
        ))
        .id();

    // Add position type component separately
    match position_type {
        PositionType::Initial => commands.entity(robot_entity).insert(InitialPosition),
        PositionType::Set => commands.entity(robot_entity).insert(SetPosition),
    };

    // Spawn the draggable circle
    commands
        .spawn((
            Mesh2d(meshes.add(Circle::new(1.0))),
            MeshMaterial2d(materials.add(ColorMaterial::from_color(color))),
            Transform::from_xyz(0.0, 0.0, 0.0),
            PositionCircle,
        ))
        .observe(|over: Trigger<Pointer<Over>>| {
            println!("Over event triggered for circle: {}", over.entity());
        })
        .set_parent(robot_entity);

    commands
        .spawn((
            Sprite {
                image: robot_texture.clone(),
                custom_size: Some(Vec2::splat(ROBOT_SIZE_METERS * PIXELS_PER_METER)),
                ..default()
            },
            Transform::from_xyz(0.0, 0.0, 0.1),
        ))
        .set_parent(robot_entity)
        .observe(|over: Trigger<Pointer<Over>>| {
            println!("Over event triggered for sprite: {}", over.entity());
        })
        .observe(on_robot_drag);

    commands
        .spawn((
            Text2d::new(config.player_number.to_string()),
            text_font.clone(),
            TextLayout::new_with_justify(text_justification),
            Transform::from_xyz(0.0, 0.0, 0.2),
            TextColor(Color::srgb(0.2, 0.2, 0.2)),
            PlayerNumber,
        ))
        .set_parent(robot_entity)
        .observe(|over: Trigger<Pointer<Over>>| {
            println!("Over event triggered for text: {}", over.entity());
        });
}

fn setup_system(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let field_texture = asset_server.load("field_simple.png");
    let robot_texture = asset_server.load("nao.png");
    let font = asset_server.load("fonts/FiraSans-Bold.ttf");
    let formation_config =
        FormationConfig::load("deploy/config/").expect("Failed to load layout config");

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
    for i in 1..=formation_config.initial_positions.len() {
        // Draw initial position robot
        draw_robot_with_type(
            &mut commands,
            &mut meshes,
            &mut materials,
            &formation_config.initial_positions.player(i as u8),
            PositionType::Initial,
            &robot_texture,
            &text_font,
            text_justification,
        );

        // Draw set position robot
        draw_robot_with_type(
            &mut commands,
            &mut meshes,
            &mut materials,
            &formation_config.set_positions.player(i as u8),
            PositionType::Set,
            &robot_texture,
            &text_font,
            text_justification,
        );
    }

    commands.spawn(Camera2d);
}
