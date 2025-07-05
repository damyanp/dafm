use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;

mod mapgen;
use mapgen::MapGenPlugin;

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
    mut run_step_event: EventWriter<mapgen::RunStepEvent>,
    mut reset_event: EventWriter<mapgen::ResetEvent>,
    mut auto_build_event: EventWriter<mapgen::AutoBuildEvent>
) {
    if keys.just_pressed(KeyCode::Space) {
        run_step_event.write(mapgen::RunStepEvent);
    }

    if keys.just_pressed(KeyCode::Enter) {
        auto_build_event.write(mapgen::AutoBuildEvent);
    }

    if keys.just_pressed(KeyCode::KeyR) {
        reset_event.write(mapgen::ResetEvent);
    }
}

fn update_labels(mut query: Query<(&mut Text2d, &TilePos)>) {
    for (mut text, tile_pos) in &mut query {
        text.0 = format!("!{}", tile_pos.x);
    }
}
