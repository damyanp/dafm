use super::GameBorder;
use avian2d::prelude::*;
use bevy::prelude::*;

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

pub fn fire(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    position: &Position,
    rotation: &Rotation,
    velocity: &LinearVelocity
) {
    let image = asset_server.load("laser.png");

    commands.spawn((
        StateScoped(crate::GameState::InGame),
        Name::new("Bullet"),
        Sprite::from_image(image.clone()),
        Bullet,
        RigidBody::Kinematic,
        *position,
        *rotation,
        LinearVelocity(velocity.0 + rotation * Vec2::Y * 500.0),
        Collider::rectangle(3.0, 6.0),
        CollidingEntities::default(),
    ));
}
