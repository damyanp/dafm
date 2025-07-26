use avian2d::{math::PI, prelude::*};
use bevy::prelude::*;

use crate::GameState;

#[derive(Component)]
pub struct Enemy;

impl Plugin for Enemy {
    fn build(&self, app: &mut App) {
        app.add_event::<SpawnEnemy>()
            .add_observer(spawn_enemy)
            .add_systems(OnEnter(GameState::InGame), start_waves)
            .add_systems(OnExit(GameState::InGame), end_waves)
            .add_systems(
                Update,
                (update_waves, update_enemies).run_if(in_state(GameState::InGame)),
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
) {
    let player_position = *player;

    for (position, velocity, mut force, mut rotation) in enemy_query {
        if velocity.length_squared() > 0.0 {
            let direction = velocity.normalize();
            *rotation = rotation.slerp(Rotation::radians(direction.to_angle() - PI * 0.5), 0.1);
        }

        let to_player = (player_position.0 - position.0).normalize();
        force.apply_force(to_player * 10000.0);
    }
}

fn spawn_enemy(spawn: Trigger<SpawnEnemy>, mut commands: Commands, assets: Res<super::GameAssets>) {
    commands.spawn((
        StateScoped(GameState::InGame),
        Name::new("Enemy"),
        Sprite::from_atlas_image(
            assets.sprite_sheet.clone(),
            TextureAtlas {
                layout: assets.sprite_sheet_layout.clone(),
                index: 6,
            },
        ),
        RigidBody::Dynamic,
        Collider::circle(16.0),
        AngularDamping(1.0),
        LinearDamping(1.0),
        ExternalForce::default().with_persistence(false),
        spawn.0,
        Enemy,
    ));
}

#[derive(Resource)]
struct Waves {
    next: f32,
}

fn start_waves(mut commands: Commands, time: Res<Time>) {
    commands.insert_resource(Waves {
        next: time.elapsed_secs() + 1.0,
    });
}

fn end_waves(mut commands: Commands) {
    commands.remove_resource::<Waves>();
}

fn update_waves(mut commands: Commands, time: Res<Time>, waves: Option<ResMut<Waves>>) {
    if let Some(mut waves) = waves {
        if waves.next <= time.elapsed_secs() {
            commands.trigger(SpawnEnemy(Position::default()));
            waves.next = time.elapsed_secs() + 30.0;
        }
    }
}
