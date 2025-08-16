use super::bullets;
use crate::GameState;
use crate::sprite_sheet::{GameSprite, SpriteSheet};
use avian2d::prelude::*;
use bevy::prelude::*;
use bevy_enhanced_input::prelude::*;
use bevy_rand::global::GlobalEntropy;
use bevy_rand::prelude::*;
use rand::RngCore;

#[derive(InputAction)]
#[action_output(f32)]
pub struct Turn;

#[derive(InputAction)]
#[action_output(f32)]
pub struct Thrust;

#[derive(InputAction)]
#[action_output(bool)]
pub struct Fire;

pub fn create_player(mut commands: Commands, sprite_sheet: Res<SpriteSheet>) {
    commands.insert_resource(Gravity::ZERO);
    commands.spawn((
        StateScoped(GameState::SpaceShooter),
        Name::new("Player"),
        sprite_sheet.sprite(GameSprite::Player),
        RigidBody::Dynamic,
        Collider::circle(16.0),
        ExternalTorque::default().with_persistence(false),
        ExternalForce::default().with_persistence(false),
        AngularDamping(1.0),
        LinearDamping(1.0),
        Position::default(),
        Player,
        bullets::StandardGun::default(),
        actions!(
            Player[
                (
                   Action::<Turn>::new(),
                   bindings![
                        (KeyCode::KeyA),
                        (KeyCode::KeyD, Negate::all())
                    ]
                ),
                (
                    Action::<Thrust>::new(),
                    bindings![
                        (KeyCode::KeyW),
                        (KeyCode::KeyS, Negate::all())
                    ]
                ),
                (
                    Action::<Fire>::new(),
                    bindings![KeyCode::Space]
                )]
        ),
    ));

    commands.insert_resource(PlayerMoveConfig::default());
}

#[derive(Component)]
pub struct Player;

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

#[allow(clippy::type_complexity)]
pub fn update_player(
    turn: Single<&Action<Turn>>,
    thrust: Single<&Action<Thrust>>,
    config: Res<PlayerMoveConfig>,
    mut rng: GlobalEntropy<WyRand>,
    mut query: Query<
        (
            &mut ExternalTorque,
            &mut ExternalForce,
            &mut AngularDamping,
            &mut LinearDamping,
            &mut Sprite,
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
        rotation,
    )) = query.single_mut()
    {
        *angular_damping = config.angular_damping;
        *linear_damping = config.linear_damping;

        let turn = ***turn;
        let thrust = ***thrust;

        torque.apply_torque(turn * config.torque);
        force.apply_force(rotation * Vec2::Y * config.thrust * thrust);

        let mut new_sprite = GameSprite::Player;
        if thrust.abs() > 0.0 {
            new_sprite = GameSprite::player_thrust(rng.next_u32());
        }

        sprite
            .texture_atlas
            .iter_mut()
            .for_each(|a| a.index = new_sprite.index());
    }
}
