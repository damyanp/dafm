use avian2d::prelude::*;
use bevy::{
    prelude::*,
    window::{PresentMode, WindowResized},
};
use bevy_ecs_tilemap::prelude::*;
use bevy_egui::EguiPlugin;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_rand::prelude::*;
use bevy_rand::{global::GlobalEntropy, plugin::EntropyPlugin};
use rand::RngCore;

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
        .add_plugins(EntropyPlugin::<WyRand>::default())
        .add_systems(Startup, startup)
        .add_systems(FixedUpdate, update_player)
        .add_systems(FixedUpdate, update_bullets)
        .add_systems(Update, on_resize_system)
        .register_type::<PlayerMoveConfig>()
        .run();
}

fn startup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    // commands.spawn((Camera2d, PanCam::default()));
    commands.spawn(Camera2d);

    let texture = asset_server.load("sprites.png");
    let layout = TextureAtlasLayout::from_grid(UVec2::splat(32), 10, 10, None, None);
    let texture_atlas_layout = texture_atlas_layouts.add(layout);

    commands.insert_resource(Gravity::ZERO);
    commands.spawn((
        Name::new("Player"),
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
        Cooldown(0),
    ));

    commands.insert_resource(PlayerMoveConfig::default());
}

#[derive(Component)]
struct GameBorder;

fn on_resize_system(
    mut commands: Commands,
    mut resize_reader: EventReader<WindowResized>,
    query: Query<Entity, With<GameBorder>>,
    mut player: Query<&mut Position, With<Player>>,
) {
    if let Some(e) = resize_reader.read().last() {
        for entity in query {
            commands.entity(entity).despawn();
        }

        for mut p in &mut player {
            *p = Position::default();
        }

        commands.spawn((
            Name::new("GameBorder 1"),
            Collider::rectangle(1.0, e.height),
            Position::from_xy(-e.width / 2.0, 0.0),
            RigidBody::Static,
            GameBorder,
        ));
        commands.spawn((
            Name::new("GameBorder 2"),
            Collider::rectangle(1.0, e.height),
            Position::from_xy(e.width / 2.0, 0.0),
            RigidBody::Static,
            GameBorder,
        ));
        commands.spawn((
            Name::new("GameBorder 3"),
            Collider::rectangle(e.width, 1.0),
            Position::from_xy(0.0, -e.height / 2.0),
            RigidBody::Static,
            GameBorder,
        ));
        commands.spawn((
            Name::new("GameBorder 4"),
            Collider::rectangle(e.width, 1.0),
            Position::from_xy(0.0, e.height / 2.0),
            RigidBody::Static,
            GameBorder,
        ));
    }
}

#[derive(Component)]
struct Player;

#[derive(Component)]
struct Cooldown(u32);

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

fn update_player(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    keys: Res<ButtonInput<KeyCode>>,
    config: Res<PlayerMoveConfig>,
    mut rng: GlobalEntropy<WyRand>,
    mut query: Query<
        (
            &mut ExternalTorque,
            &mut ExternalForce,
            &mut AngularDamping,
            &mut LinearDamping,
            &mut Sprite,
            &mut Cooldown,
            &Transform,
            &LinearVelocity,
            &Rotation,
        ),
        With<Player>,
    >,
) {
    if let Ok((
        mut torque,
        mut force,
        mut angular_damping,
        mut linear_damping,
        mut sprite,
        mut cooldown,
        transform,
        velocity,
        rotation,
    )) = query.single_mut()
    {
        *angular_damping = config.angular_damping;
        *linear_damping = config.linear_damping;

        if keys.pressed(KeyCode::ArrowLeft) || keys.pressed(KeyCode::KeyA) {
            torque.apply_torque(config.torque);
        }
        if keys.pressed(KeyCode::ArrowRight) || keys.pressed(KeyCode::KeyD) {
            torque.apply_torque(-config.torque);
        }

        let mut new_index = 2;
        if keys.pressed(KeyCode::ArrowUp) || keys.pressed(KeyCode::KeyW) {
            force.apply_force((transform.rotation * Vec3::Y * config.thrust).truncate());
            new_index = 3 + rng.next_u32() % 2;
        }

        sprite
            .texture_atlas
            .iter_mut()
            .for_each(|a| a.index = new_index as usize);

        if cooldown.0 > 0 {
            cooldown.0 -= 1;
        }

        if cooldown.0 == 0 && keys.pressed(KeyCode::Space) {
            let image = asset_server.load("laser.png");

            commands.spawn((
                Name::new("Bullet"),
                Sprite::from_image(image.clone()),
                Bullet,
                RigidBody::Kinematic,
                Position::new(transform.translation.truncate()),
                rotation.clone(),
                LinearVelocity(velocity.0 + (transform.rotation * Vec3::Y * 500.0).truncate()),
                Collider::rectangle(3.0, 6.0),
                CollidingEntities::default(),
            ));

            cooldown.0 = 5;
        }
    }
}

#[derive(Component)]
struct Bullet;

fn update_bullets(
    mut commands: Commands,
    bullets: Query<(Entity, &CollidingEntities), With<Bullet>>,
    walls: Query<Entity, With<GameBorder>>,
) {
    for (bullet, colliding_entities) in bullets {
        for colliding_entity in colliding_entities.iter() {
            if walls.get(*colliding_entity).is_ok() {
                commands.entity(bullet).despawn();
                break;
            }
        }
    }
}
