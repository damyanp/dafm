use super::{GameBorder, player};
use crate::GameState;
use avian2d::prelude::*;
use bevy::prelude::*;
use bevy_enhanced_input::prelude::*;

pub struct Bullets;
impl Plugin for Bullets {
    fn build(&self, app: &mut App) {
        app.add_event::<Damage>().add_systems(
            FixedUpdate,
            (update_standard_gun).run_if(in_state(GameState::InGame)),
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
    query: Query<(&mut StandardGun, &Position, &Rotation, &LinearVelocity)>,
) {
    for (mut gun, position, rotation, velocity) in query {
        if gun.cooldown > 0 {
            gun.cooldown -= 1;
        }

        if gun.cooldown == 0 && ***fire {
            commands
                .spawn((
                    StateScoped(crate::GameState::InGame),
                    Name::new("Bullet"),
                    Sprite::from_image(assets.laser.clone()),
                    Bullet,
                    RigidBody::Kinematic,
                    *position,
                    *rotation,
                    LinearVelocity(velocity.0 + rotation * Vec2::Y * 500.0),
                    Collider::rectangle(3.0, 6.0),
                    CollisionEventsEnabled,
                    Mass(0.01),
                ))
                .observe(observe_bullets);
            gun.cooldown = 5;
        }
    }
}

#[derive(Component)]
pub struct Bullet;

#[derive(Component)]
pub struct Damageable;

#[derive(Event)]
pub struct Damage;

fn observe_bullets(
    trigger: Trigger<OnCollisionStart>,
    borders: Query<&GameBorder>,
    damageables: Query<&Damageable>,
    mut commands: Commands,
) {
    if borders.contains(trigger.collider) {
        commands.entity(trigger.target()).despawn();
    }

    if damageables.contains(trigger.collider) {
        commands.trigger_targets(Damage, trigger.collider);
        commands.entity(trigger.target()).despawn();
    }
}
