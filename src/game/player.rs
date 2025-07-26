use avian2d::prelude::*;
use bevy::prelude::*;
use bevy_rand::global::GlobalEntropy;
use bevy_rand::prelude::*;
use rand::RngCore;

use crate::game::Bullet;
use crate::GameState;

pub fn create_player(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    let texture = asset_server.load("sprites.png");
    let layout = TextureAtlasLayout::from_grid(UVec2::splat(32), 10, 10, None, None);
    let texture_atlas_layout = texture_atlas_layouts.add(layout);

    commands.insert_resource(Gravity::ZERO);
    commands.spawn((
        StateScoped(GameState::InGame),
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
pub struct Player;

#[derive(Component)]
pub struct Cooldown(u32);

#[derive(Resource, Reflect)]
#[reflect(Resource)]
pub struct PlayerMoveConfig {
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

pub fn update_player(
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
                StateScoped(GameState::InGame),
                Name::new("Bullet"),
                Sprite::from_image(image.clone()),
                Bullet,
                RigidBody::Kinematic,
                Position::new(transform.translation.truncate()),
                *rotation,
                LinearVelocity(velocity.0 + (transform.rotation * Vec3::Y * 500.0).truncate()),
                Collider::rectangle(3.0, 6.0),
                CollidingEntities::default(),
            ));

            cooldown.0 = 5;
        }
    }
}
