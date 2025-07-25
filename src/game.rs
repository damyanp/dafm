use crate::GameState;
use avian2d::prelude::*;
use bevy::{prelude::*, window::WindowResized};
use bevy_enhanced_input::prelude::*;

mod bullets;
mod player;

pub struct Game;

impl Plugin for Game {
    fn build(&self, app: &mut App) {
        app.add_plugins(bullets::Bullets)
            .add_systems(Startup, load_assets)
            .register_type::<player::PlayerMoveConfig>()
            .add_input_context::<player::Player>()
            .add_systems(
                OnEnter(GameState::InGame),
                (player::create_player, setup_game_borders),
            )
            .add_systems(
                FixedUpdate,
                (player::update_player).run_if(in_state(GameState::InGame)),
            )
            .add_systems(
                Update,
                (update_game_borders, check_for_exit).run_if(in_state(GameState::InGame)),
            );
    }
}

#[derive(Resource)]
pub struct GameAssets {
    pub sprite_sheet: Handle<Image>,
    pub sprite_sheet_layout: Handle<TextureAtlasLayout>,
    pub laser: Handle<Image>,
}

fn load_assets(
    mut commands: Commands,
    mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
    asset_server: Res<AssetServer>,
) {
    let layout = TextureAtlasLayout::from_grid(UVec2::splat(32), 10, 10, None, None);
    let sprite_sheet_layout = texture_atlas_layouts.add(layout);

    commands.insert_resource(GameAssets {
        sprite_sheet: asset_server.load("sprites.png"),
        sprite_sheet_layout,
        laser: asset_server.load("laser.png"),
    })
}

fn check_for_exit(mut commands: Commands, mut keys: ResMut<ButtonInput<KeyCode>>) {
    if keys.clear_just_released(KeyCode::Escape) {
        commands.set_state(GameState::MainMenu);
    }
}

#[derive(Component)]
struct GameBorder;

fn setup_game_borders(
    commands: Commands,
    window: Single<&Window>,
    query: Query<Entity, With<GameBorder>>,
    player: Query<&mut Position, With<player::Player>>,
) {
    let r = &window.resolution;

    create_game_borders(commands, query, player, r.width(), r.height());
}

fn update_game_borders(
    commands: Commands,
    mut resize_reader: EventReader<WindowResized>,
    query: Query<Entity, With<GameBorder>>,
    player: Query<&mut Position, With<player::Player>>,
) {
    if let Some(e) = resize_reader.read().last() {
        create_game_borders(commands, query, player, e.width, e.height);
    }
}

fn create_game_borders(
    mut commands: Commands,
    query: Query<Entity, With<GameBorder>>,
    mut player: Query<&mut Position, With<player::Player>>,
    width: f32,
    height: f32,
) {
    for entity in query {
        commands.entity(entity).despawn();
    }

    for mut p in &mut player {
        *p = Position::default();
    }

    commands.spawn((
        StateScoped(GameState::InGame),
        Name::new("GameBorder 1"),
        Collider::rectangle(1.0, height),
        Position::from_xy(-width / 2.0, 0.0),
        RigidBody::Static,
        GameBorder,
    ));
    commands.spawn((
        StateScoped(GameState::InGame),
        Name::new("GameBorder 2"),
        Collider::rectangle(1.0, height),
        Position::from_xy(width / 2.0, 0.0),
        RigidBody::Static,
        GameBorder,
    ));
    commands.spawn((
        StateScoped(GameState::InGame),
        Name::new("GameBorder 3"),
        Collider::rectangle(width, 1.0),
        Position::from_xy(0.0, -height / 2.0),
        RigidBody::Static,
        GameBorder,
    ));
    commands.spawn((
        StateScoped(GameState::InGame),
        Name::new("GameBorder 4"),
        Collider::rectangle(width, 1.0),
        Position::from_xy(0.0, height / 2.0),
        RigidBody::Static,
        GameBorder,
    ));
}
