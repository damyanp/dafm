use avian2d::prelude::*;
use bevy::{prelude::*, render::mesh::RectangleMeshBuilder, window::PresentMode};
use bevy_ecs_tilemap::prelude::*;
use bevy_egui::EguiPlugin;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_pancam::{PanCam, PanCamPlugin};

mod terrain;

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(ImagePlugin::default_nearest())
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        present_mode: PresentMode::Immediate,
                        ..default()
                    }),
                    ..default()
                }),
        )
        .add_plugins(PhysicsPlugins::default())
        .add_plugins(PhysicsDebugPlugin::default())
        .add_plugins(EguiPlugin::default())
        .add_plugins(WorldInspectorPlugin::new())
        // .add_plugins(PanCamPlugin::default())
        .add_plugins(TilemapPlugin)
        // .add_plugins(terrain::TerrainPlugin)
        .add_systems(Startup, startup)
        .add_systems(Update, update)
        .run();
}

fn startup(
    mut commands: Commands,
    mut asset_server: Res<AssetServer>,
    mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    // commands.spawn((Camera2d, PanCam::default()));
    commands.spawn(Camera2d);

    let texture = asset_server.load("sprites.png");
    let layout = TextureAtlasLayout::from_grid(UVec2::splat(32), 10, 10, None, None);
    let texture_atlas_layout = texture_atlas_layouts.add(layout);

    commands.insert_resource(Gravity::ZERO);
    commands.spawn((
        Sprite::from_atlas_image(
            texture,
            TextureAtlas {
                layout: texture_atlas_layout,
                index: 0,
            },
        ),
        RigidBody::Dynamic,
        Collider::circle(16.0),
        ExternalTorque::default().with_persistence(false),
        ExternalForce::default().with_persistence(false),
        AngularDamping(1.0),
        LinearDamping(1.0),
        Player,
    ));
    commands.spawn((
        Collider::rectangle(5.0, 200.0),
        Position::from_xy(-100.0, 0.0),
        RigidBody::Static,
    ));
    commands.spawn((
        Collider::rectangle(5.0, 200.0),
        Position::from_xy(100.0, 0.0),
        RigidBody::Static,
    ));
    commands.spawn((
        Collider::rectangle(200.0, 5.0),
        Position::from_xy(0.0, -100.0),
        RigidBody::Static,
    ));
    commands.spawn((
        Collider::rectangle(200.0, 5.0),
        Position::from_xy(0.0, 100.0),
        RigidBody::Static,
    ));
}

#[derive(Component)]
struct Player;

fn update(
    keys: Res<ButtonInput<KeyCode>>,
    mut query: Query<(&mut ExternalTorque, &mut ExternalForce, &Transform), With<Player>>,
) {

    const TORQUE:f32 = 100000.0;
    const THRUST:f32 = 100000.0;

    let (mut torque, mut force, transform) = query.single_mut().unwrap();
    if keys.pressed(KeyCode::ArrowLeft) {
        torque.apply_torque(TORQUE);
    }
    if keys.pressed(KeyCode::ArrowRight) {
        torque.apply_torque(-TORQUE);
    }
    if keys.pressed(KeyCode::ArrowUp) {
        force.apply_force((transform.rotation * Vec3::Y * THRUST).truncate());
    }
}
