use super::{GameBorder, player};
use crate::GameState;
use avian2d::prelude::*;
use bevy::prelude::*;
use bevy_enhanced_input::prelude::*;

pub struct Bullets;
impl Plugin for Bullets {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            (update_standard_gun, update_bullets).run_if(in_state(GameState::InGame)),
        );
    }
}

#[derive(Component, Default)]
pub struct StandardGun {
    cooldown: u32,
}

fn update_standard_gun(
    mut commands: Commands,
    assets: Res<super::GameAssets>,
    fire: Single<&Action<player::Fire>>,
    query: Query<(
        &mut StandardGun,
        &Position,
        &Rotation,
        &LinearVelocity,
    )>,
) {
    for (mut gun, position, rotation, velocity) in query {
        if gun.cooldown > 0 {
            gun.cooldown -= 1;
        }

        if gun.cooldown == 0 && ***fire {
            commands.spawn((
                StateScoped(crate::GameState::InGame),
                Name::new("Bullet"),
                Sprite::from_image(assets.laser.clone()),
                Bullet,
                RigidBody::Kinematic,
                *position,
                *rotation,
                LinearVelocity(velocity.0 + rotation * Vec2::Y * 500.0),
                Collider::rectangle(3.0, 6.0),
                CollidingEntities::default(),
            ));
            gun.cooldown = 5;
        }
    }
}

#[derive(Component)]
pub struct Bullet;

pub fn update_bullets(
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
