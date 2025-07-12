use super::mapgen;
use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;

pub struct MapGenPlugin;

#[derive(Event, PartialEq)]
pub enum MapGenControlEvent {
    Step,
    AutoStep,
    Build,
    Reset,
}

#[derive(Component)]
struct MapGenerator {
    generator: mapgen::Generator,
    auto_step: bool,
}

impl Plugin for MapGenPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, startup)
            .add_event::<MapGenControlEvent>()
            .add_systems(Update, (manage_generator, update).chain())
            .add_systems(Update, mapgen_controls);
    }
}

fn startup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let texture = asset_server.load("kentangpixel/SummerFloor.png");

    let map_size = TilemapSize { x: 100, y: 60 };

    let tilemap_entity = commands.spawn_empty().id();

    let mut tile_storage = TileStorage::empty(map_size);

    let tile_size = TilemapTileSize { x: 32.0, y: 32.0 };
    let grid_size = tile_size.into();
    let map_type = TilemapType::default();

    for x in 0..map_size.x {
        for y in 0..map_size.y {
            let tile_pos = TilePos { x, y };
            let tile_entity = commands
                .spawn(TileBundle {
                    position: tile_pos,
                    tilemap_id: TilemapId(tilemap_entity),
                    texture_index: TileTextureIndex(0),
                    ..default()
                })
                .id();
            tile_storage.set(&tile_pos, tile_entity);
        }
    }

    commands.entity(tilemap_entity).insert(TilemapBundle {
        grid_size,
        map_type,
        size: map_size,
        storage: tile_storage,
        texture: TilemapTexture::Single(texture),
        tile_size,
        anchor: TilemapAnchor::Center,
        ..default()
    });
}

fn mapgen_controls(
    keys: Res<ButtonInput<KeyCode>>,
    mut control_event: EventWriter<MapGenControlEvent>,
) {
    if keys.just_pressed(KeyCode::Space) {
        control_event.write(MapGenControlEvent::Step);
    }

    if keys.just_pressed(KeyCode::Enter) {
        control_event.write(MapGenControlEvent::AutoStep);
    }

    if keys.just_pressed(KeyCode::KeyR) {
        control_event.write(MapGenControlEvent::Reset);
    }

    if keys.just_pressed(KeyCode::KeyB) {
        control_event.write(MapGenControlEvent::Build);
    }
}

#[derive(Component)]
struct TileLabel(Entity);

#[allow(clippy::type_complexity)]
fn manage_generator(
    mut commands: Commands,
    mut control_events: EventReader<MapGenControlEvent>,
    mut tile_maps: Query<(
        Entity,
        &TileStorage,
        &Transform,
        &TilemapSize,
        &TilemapGridSize,
        &TilemapTileSize,
        &TilemapType,
        &TilemapAnchor,
        Option<&mut MapGenerator>,
    )>,
    mut tiles: Query<(&mut TileTextureIndex, &TilePos, Option<&TileLabel>)>,
) {
    for control_event in control_events.read() {
        if *control_event == MapGenControlEvent::Reset {
            // Reset the tiles
            for (mut tile_texture_index, _, label) in &mut tiles {
                tile_texture_index.0 = 0;
                if let Some(TileLabel(label)) = label {
                    commands.entity(*label).despawn();
                }
            }
        }

        if *control_event == MapGenControlEvent::Build {
            for (_, _, _, _, _, _, _, _, mut generator) in &mut tile_maps {
                if let Some(generator) = &mut generator {
                    generator.generator.reset();
                }
            }
        }

        for tile_map in &tile_maps {
            match control_event {
                MapGenControlEvent::Step
                | MapGenControlEvent::AutoStep
                | MapGenControlEvent::Build => {
                    let (
                        entity,
                        storage,
                        transform,
                        map_size,
                        grid_size,
                        tile_size,
                        map_type,
                        anchor,
                        generator,
                    ) = tile_map;

                    if generator.is_some() {
                        continue;
                    }

                    // Add labels over all the tiles
                    for tile_entity in storage.iter().flatten() {
                        let (_, tile_pos, _) = tiles.get(*tile_entity).unwrap();
                        let tile_center = tile_pos
                            .center_in_world(map_size, grid_size, tile_size, map_type, anchor)
                            .extend(1.0);
                        let transform = *transform * Transform::from_translation(tile_center);

                        let label = commands
                            .spawn((
                                Text2d::new("-"),
                                TextFont {
                                    font_size: 10.0,
                                    ..default()
                                },
                                TextColor(Color::WHITE),
                                TextLayout::new_with_justify(JustifyText::Center),
                                transform,
                            ))
                            .id();

                        commands.entity(*tile_entity).insert(TileLabel(label));
                    }
                    commands.entity(entity).insert(MapGenerator {
                        generator: mapgen::Generator::new(map_size),
                        auto_step: false,
                    });
                }
                MapGenControlEvent::Reset => {
                    let tile_map_entity = tile_map.0;
                    commands.entity(tile_map_entity).remove::<MapGenerator>();
                }
            }
        }
    }
}

fn update(
    mut commands: Commands,
    mut control_events: EventReader<MapGenControlEvent>,
    mut generators: Query<(&mut MapGenerator, &TileStorage)>,
    mut tiles: Query<(&mut TileTextureIndex, &TileLabel)>,
    mut labels: Query<&mut Text2d>,
) {
    let mut do_step = false;
    let mut do_build_now = false;

    for control_event in control_events.read() {
        match control_event {
            MapGenControlEvent::Step => do_step = true,
            MapGenControlEvent::AutoStep => {
                for (mut generator, _) in &mut generators {
                    generator.auto_step = true;
                }
            }
            MapGenControlEvent::Build => do_build_now = true,
            MapGenControlEvent::Reset => {}
        }
    }

    do_step = do_build_now || do_step || generators.iter().any(|(g, _)| g.auto_step);
    let did_a_step = do_step;

    while do_step {
        let all_done = generators.iter_mut().all(|(mut g, _)| g.generator.step());
        do_build_now = do_build_now && !all_done;
        do_step = do_build_now;
    }

    if did_a_step {
        for (map_generator, storage) in generators {
            let generated_map = map_generator.generator.get();
            for generated_tile in generated_map {
                let tile_entity = storage.get(&generated_tile.pos).unwrap();
                let (mut texture_index, TileLabel(label)) = tiles.get_mut(tile_entity).unwrap();

                match generated_tile.state {
                    mapgen::TileState::Collapsed(i) => {
                        texture_index.0 = i;
                        commands.entity(*label).insert(Visibility::Hidden);
                        labels.get_mut(*label).unwrap().0 = format!("{}", i);
                    }
                    mapgen::TileState::Options(count) => {
                        commands.entity(*label).insert(Visibility::Visible);
                        labels.get_mut(*label).unwrap().0 = if count == f32::MAX {
                            "-".to_string()
                        } else {
                            format!("{:.0}", count * 100.0)
                        };
                    }
                }
            }
        }
    }
}
