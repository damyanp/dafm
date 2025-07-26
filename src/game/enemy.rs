use avian2d::prelude::*;
use bevy::prelude::*;

use crate::GameState;

pub struct Enemy;
impl Plugin for Enemy {
    fn build(&self, app: &mut App) {
        app.add_event::<SpawnEnemy>()
            .add_observer(spawn_enemy)
            .add_systems(OnEnter(GameState::InGame), start_waves)
            .add_systems(OnExit(GameState::InGame), end_waves)
            .add_systems(Update, update_waves);
    }
}

#[derive(Event)]
struct SpawnEnemy(Position);

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
            waves.next = time.elapsed_secs() + 2.0;
        }
    }
}
