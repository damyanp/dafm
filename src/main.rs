use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;

mod mapgen;
mod mapgen_viz;
use mapgen_viz::MapGenPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        .add_plugins(TilemapPlugin)
        .add_plugins(MapGenPlugin)
        .add_systems(Startup, startup)
        .add_systems(Update, update_labels)
        .add_systems(Update, mapgen_controls)
        .run();
}

fn startup(mut commands: Commands) {
    commands.spawn(Camera2d);
}

fn mapgen_controls(
    keys: Res<ButtonInput<KeyCode>>,
    mut control_event: EventWriter<mapgen_viz::MapGenControlEvent>,
) {
    if keys.just_pressed(KeyCode::Space) {
        control_event.write(mapgen_viz::MapGenControlEvent::Step);
    }

    if keys.just_pressed(KeyCode::Enter) {
        control_event.write(mapgen_viz::MapGenControlEvent::AutoStep);
    }

    if keys.just_pressed(KeyCode::KeyR) {
        control_event.write(mapgen_viz::MapGenControlEvent::Reset);
    }

    if keys.just_pressed(KeyCode::KeyB) {
        control_event.write(mapgen_viz::MapGenControlEvent::Build);
    }
}

fn update_labels(mut query: Query<(&mut Text2d, &TilePos)>) {
    for (mut text, tile_pos) in &mut query {
        text.0 = format!("!{}", tile_pos.x);
    }
}
