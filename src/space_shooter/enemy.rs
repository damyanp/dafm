use super::bullets::{Damage, Damageable};
use crate::{
    GameState,
    sprite_sheet::{GameSprite, SpriteSheet},
};
use avian2d::{math::PI, prelude::*};
#[allow(unused_imports)]
use bevy::{
    color::palettes::css::{BLUE, GREEN, PURPLE, YELLOW},
    prelude::*,
};
use bevy_rand::{global::GlobalEntropy, prelude::WyRand};

#[derive(Component)]
pub struct Enemy;

#[derive(Component)]
struct DestroyedEnemy(u32);

impl Plugin for Enemy {
    fn build(&self, app: &mut App) {
        app.add_event::<SpawnEnemy>()
            .add_observer(spawn_enemy)
            .add_systems(OnEnter(GameState::SpaceShooter), start_waves)
            .add_systems(OnExit(GameState::SpaceShooter), end_waves)
            .add_systems(
                Update,
                (update_waves, update_enemies, update_destroyed_enemies)
                    .run_if(in_state(GameState::SpaceShooter)),
            );
    }
}

#[derive(Event)]
struct SpawnEnemy(Position);

fn update_enemies(
    enemy_query: Query<
        (
            &Position,
            &LinearVelocity,
            &mut ExternalForce,
            &mut Rotation,
        ),
        With<Enemy>,
    >,
    player: Single<&Position, With<super::player::Player>>,
    mut _gizmos: Gizmos,
) {
    let player_position = *player;

    for (position, velocity, mut force, mut rotation) in enemy_query {
        if velocity.length_squared() > 0.0 {
            let direction = velocity.normalize();
            *rotation = rotation.slerp(Rotation::radians(direction.to_angle() - PI * 0.5), 0.1);
        }

        let to_player = (player_position.0 - position.0).normalize();

        let mut thrust = |f, _color| {
            // _gizmos.ray_2d(**position, f * 0.01, _color);
            force.apply_force(f);
        };

        if velocity.0.length_squared() > 0.0 {
            let across = Vec3::Z.cross(velocity.0.extend(0.0)).truncate();
            let d = across.normalize().dot(to_player);
            thrust(across * (d * 1000.0), BLUE);

            thrust(
                velocity.0.normalize() * -(velocity.0.length() - 200.0).max(0.0) * 1000.0,
                PURPLE,
            );
        }
        thrust(to_player * 10000.0, YELLOW);

        // _gizmos.ray_2d(**position, velocity.0, GREEN);
    }
}

fn update_destroyed_enemies(
    mut commands: Commands,
    enemy_query: Query<(&mut ExternalTorque, &mut DestroyedEnemy, Entity)>,
) {
    for (mut torque, mut enemy, entity) in enemy_query {
        torque.apply_torque(10000.0);

        enemy.0 -= 1;

        if enemy.0 == 0 {
            commands.entity(entity).despawn();
        }
    }
}

fn spawn_enemy(spawn: Trigger<SpawnEnemy>, mut commands: Commands, sprite_sheet: Res<SpriteSheet>) {
    commands
        .spawn((
            StateScoped(GameState::SpaceShooter),
            Name::new("Enemy"),
            sprite_sheet.sprite(GameSprite::Enemy),
            RigidBody::Dynamic,
            Collider::circle(16.0),
            AngularDamping(1.0),
            LinearDamping(0.0),
            ExternalForce::default().with_persistence(false),
            spawn.0,
            Enemy,
            Damageable,
        ))
        .observe(observe_damage);
}

fn observe_damage(trigger: Trigger<Damage>, mut commands: Commands) {
    // commands.entity(trigger.target()).despawn();
    commands
        .entity(trigger.target())
        .remove::<(Enemy, Collider)>()
        .insert(DestroyedEnemy(300));
}

#[derive(Resource, Reflect)]
#[reflect(Resource)]
struct Waves {
    next: f32,
    wave_number: u32,
}

fn start_waves(mut commands: Commands, time: Res<Time>) {
    commands.insert_resource(Waves {
        next: time.elapsed_secs() + 1.0,
        wave_number: 1,
    });
}

fn end_waves(mut commands: Commands) {
    commands.remove_resource::<Waves>();
}

fn update_waves(
    mut commands: Commands,
    time: Res<Time>,
    waves: Option<ResMut<Waves>>,
    mut rng: GlobalEntropy<WyRand>,
    enemies: Query<Entity, With<Enemy>>,
) {
    if let Some(mut waves) = waves
        && waves.next <= time.elapsed_secs()
        && enemies.is_empty()
    {
        let circle = Circle::new(128.0);

        for _ in 0..waves.wave_number {
            commands.trigger(SpawnEnemy(circle.sample_interior(rng.as_mut()).into()));
        }
        waves.wave_number += 1;
        waves.next = time.elapsed_secs() + 5.0;
    }
}
