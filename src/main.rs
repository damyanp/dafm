use avian2d::prelude::*;
use bevy::{
    prelude::*,
    render::mesh::RectangleMeshBuilder,
    window::{PresentMode, WindowResized},
};
use bevy_ecs_tilemap::prelude::*;
use bevy_egui::EguiPlugin;
use bevy_inspector_egui::quick::{ResourceInspectorPlugin, WorldInspectorPlugin};
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
        // .add_plugins(PhysicsDebugPlugin::default())
        .add_plugins(EguiPlugin::default())
        .add_plugins(WorldInspectorPlugin::new())
        // .add_plugins(ResourceInspectorPlugin::<PlayerMoveConfig>::default())
        // .add_plugins(PanCamPlugin::default())
        .add_plugins(TilemapPlugin)
        // .add_plugins(terrain::TerrainPlugin)
        .add_systems(Startup, startup)
        .add_systems(FixedUpdate, update)
        .add_systems(Update, on_resize_system)
        .register_type::<PlayerMoveConfig>()
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

    commands.insert_resource(PlayerMoveConfig::default());
}

#[derive(Component)]
struct GameBorder;

fn on_resize_system(
    mut commands: Commands,
    mut resize_reader: EventReader<WindowResized>,
    mut query: Query<Entity, With<GameBorder>>,
) {
    for e in resize_reader.read() {
        for entity in query {
            commands.entity(entity).despawn();
        }

        commands.spawn((
            Collider::rectangle(1.0, e.height),
            Position::from_xy(-e.width / 2.0, 0.0),
            RigidBody::Static,
            GameBorder,
        ));
        commands.spawn((
            Collider::rectangle(1.0, e.height),
            Position::from_xy(e.width / 2.0, 0.0),
            RigidBody::Static,
            GameBorder,
        ));
        commands.spawn((
            Collider::rectangle(e.width, 1.0),
            Position::from_xy(0.0, -e.height / 2.0),
            RigidBody::Static,
            GameBorder,
        ));
        commands.spawn((
            Collider::rectangle(e.width, 1.0),
            Position::from_xy(0.0, e.height / 2.0),
            RigidBody::Static,
            GameBorder,
        ));
    }
}

#[derive(Component)]
struct Player;

#[derive(Resource, Reflect)]
#[reflect(Resource)]
struct PlayerMoveConfig {
    torque: f32,
    thrust: f32,
    angular_damping: AngularDamping,
    linear_damping: LinearDamping,
}

impl Default for PlayerMoveConfig {
    fn default() -> Self {
        Self {
            torque: 5000000.0,
            thrust: 1000000.0,
            angular_damping: AngularDamping(8.0),
            linear_damping: LinearDamping(8.0),
        }
    }
}

fn update(
    keys: Res<ButtonInput<KeyCode>>,
    config: Res<PlayerMoveConfig>,
    mut query: Query<
        (
            &mut ExternalTorque,
            &mut ExternalForce,
            &mut AngularDamping,
            &mut LinearDamping,
            &mut Sprite,
            &Transform,
        ),
        With<Player>,
    >,
) {
    let (mut torque, mut force, mut angular_damping, mut linear_damping, mut sprite, transform) =
        query.single_mut().unwrap();

    *angular_damping = config.angular_damping;
    *linear_damping = config.linear_damping;

    if keys.pressed(KeyCode::ArrowLeft) {
        torque.apply_torque(config.torque);
    }
    if keys.pressed(KeyCode::ArrowRight) {
        torque.apply_torque(-config.torque);
    }

    let mut new_index = 0;
    if keys.pressed(KeyCode::ArrowUp) {
        force.apply_force((transform.rotation * Vec3::Y * config.thrust).truncate());
        new_index = 1;
    }

    sprite
        .texture_atlas
        .iter_mut()
        .for_each(|a| a.index = new_index);
}
