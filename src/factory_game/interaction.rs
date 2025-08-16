use bevy::{
    input::{ButtonState, common_conditions::input_just_pressed, keyboard::KeyboardInput},
    prelude::*,
};
use bevy_ecs_tilemap::prelude::*;
use bevy_egui::input::{EguiWantsInput, egui_wants_any_input};
use bevy_pancam::PanCam;

use crate::{
    GameState,
    factory_game::{
        BaseLayer, BaseLayerEntityDespawned, ConveyorSystems, MapConfig, bridge::BridgeTool,
        conveyor_belts::ConveyorBeltTool, distributor::DistributorTool, generator::GeneratorTool,
        sink::SinkTool,
    },
    sprite_sheet::{GameSprite, SpriteSheet},
};

pub struct ConveyorInteractionPlugin;
impl Plugin for ConveyorInteractionPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Tools>()
            .add_systems(OnEnter(GameState::FactoryGame), (startup, setup_tools))
            .add_systems(
                OnExit(GameState::FactoryGame),
                (reset_cursor, cleanup_tools),
            )
            .add_systems(
                Update,
                (
                    (
                        (
                            track_mouse,
                            on_click.run_if(input_just_pressed(MouseButton::Left)),
                        )
                            .chain(),
                        select_tool,
                    )
                        .in_set(ConveyorSystems::TileGenerator)
                        .run_if(not(egui_wants_any_input)),
                    update_hovered_tile
                        .in_set(ConveyorSystems::TileUpdater)
                        .run_if(resource_exists_and_changed::<Tools>),
                    give_control_to_egui
                        .run_if(in_state(GameState::FactoryGame))
                        .run_if(
                            resource_exists_and_changed::<Tools>
                                .or(resource_exists_and_changed::<EguiWantsInput>),
                        ),
                ),
            );
    }
}

fn startup(mut commands: Commands, sprite_sheet: Res<SpriteSheet>, config: Res<MapConfig>) {
    let interaction_layer = commands
        .spawn(make_interaction_layer(&config, sprite_sheet.image()))
        .id();

    commands.spawn((
        StateScoped(GameState::FactoryGame),
        Name::new("HoveredTile"),
        HoveredTile,
        TileBundle {
            texture_index: GameSprite::BlankSquare.tile_texture_index(),
            tilemap_id: TilemapId(interaction_layer),
            ..default()
        },
    ));
}

fn reset_cursor(windows: Query<&mut Window>) {
    for mut window in windows {
        window.cursor_options.visible = true;
    }
}

fn give_control_to_egui(
    windows: Query<&mut Window>,
    egui_wants_input: Res<EguiWantsInput>,
    mut hovered_tile_visible: Single<&mut TileVisible, With<HoveredTile>>,
    mut pancam: Single<&mut PanCam>,
    tools: Res<Tools>,
) {
    let egui = egui_wants_input.wants_any_input();
    let tool = tools.current_tool().is_some();

    for mut window in windows {
        window.cursor_options.visible = egui || !tool;
    }

    hovered_tile_visible.0 = !egui && tool;
    pancam.enabled = !egui;
}

fn update_hovered_tile(
    q: Single<(&mut TileTextureIndex, &mut TileFlip), With<HoveredTile>>,
    tools: Res<Tools>,
) {
    let (mut texture_index, mut flip) = q.into_inner();

    if let Some((t, f)) = tools.get_sprite_flip() {
        (*texture_index, *flip) = (t.tile_texture_index(), f);
    }
}

#[allow(clippy::type_complexity)]
fn track_mouse(
    mut cursor_moved: EventReader<CursorMoved>,
    camera_query: Single<(&GlobalTransform, &Camera)>,
    interaction_layer: Single<
        (
            &TilemapSize,
            &TilemapGridSize,
            &TilemapTileSize,
            &TilemapType,
            &TilemapAnchor,
        ),
        With<InteractionLayer>,
    >,
    mut hovered_tile: Single<&mut TilePos, With<HoveredTile>>,
) {
    if let Some(e) = cursor_moved.read().last() {
        let (global_transform, camera) = *camera_query;
        if let Ok(p) = camera.viewport_to_world_2d(global_transform, e.position) {
            let (size, grid_size, tile_size, map_type, anchor) = *interaction_layer;

            if let Some(tile_pos) =
                TilePos::from_world_pos(&p, size, grid_size, tile_size, map_type, anchor)
            {
                **hovered_tile = tile_pos;
            }
        }
    }
}

#[allow(clippy::type_complexity)]
fn on_click(
    mut commands: Commands,
    tile_pos: Single<&TilePos, With<HoveredTile>>,
    mut storage: Single<&mut TileStorage, With<BaseLayer>>,
    mut despawned_event: EventWriter<BaseLayerEntityDespawned>,
    tools: Res<Tools>,
) {
    let tile_pos = *tile_pos;

    if let Some(old_entity) = storage.remove(tile_pos) {
        commands.entity(old_entity).despawn();
        despawned_event.write(BaseLayerEntityDespawned(*tile_pos));
    }
    if let Some(tool) = tools.current_tool()
        && tool.creates_entity()
    {
        let entity = commands
            .spawn((StateScoped(GameState::FactoryGame), BaseLayer, *tile_pos))
            .id();
        storage.set(tile_pos, entity);

        tool.configure_new_entity(commands.entity(entity));
    }
}

#[derive(Resource, Default, Reflect)]
#[reflect(Resource)]
pub struct Tools {
    current_tool: Option<usize>,
    #[reflect(ignore)]
    tools: Vec<ToolEntry>,
}

pub struct ToolEntry {
    slot: u32,
    tool: Box<dyn Tool>,
}

impl Tools {
    pub fn add(&mut self, slot: u32, tool: Box<dyn Tool>) {
        self.tools.push(ToolEntry { slot, tool });
        self.tools.sort_by_key(|t| t.slot);
    }

    pub fn get_sprite_flip(&self) -> Option<(GameSprite, TileFlip)> {
        self.current_tool
            .map(|i| self.tools[i].tool.get_sprite_flip())
    }

    pub fn set_no_tool(&mut self) {
        self.current_tool = None
    }

    pub fn set_tool(&mut self, slot: u32) {
        self.current_tool = self
            .tools
            .iter()
            .enumerate()
            .find(|(_, tool)| tool.slot == slot)
            .map(|(index, _)| index);
    }

    pub fn next_variant(&mut self) {
        if let Some(current_tool) = self.current_tool {
            self.tools[current_tool].tool.next_variant();
        }
    }

    pub fn current_tool(&self) -> Option<&dyn Tool> {
        self.current_tool
            .map(|index| self.tools[index].tool.as_ref())
    }

    pub fn tools(&self) -> &Vec<ToolEntry> {
        &self.tools
    }
}

impl ToolEntry {
    pub fn slot(&self) -> u32 {
        self.slot
    }
}

pub trait Tool: Sync + Send {
    fn get_sprite_flip(&self) -> (GameSprite, TileFlip);
    fn next_variant(&mut self) {}

    fn creates_entity(&self) -> bool {
        true
    }
    fn configure_new_entity(&self, commands: EntityCommands);
}

fn select_tool(mut tools: ResMut<Tools>, mut key_events: EventReader<KeyboardInput>) {
    for e in key_events.read() {
        if e.state == ButtonState::Pressed {
            match e.key_code {
                KeyCode::Backquote => tools.set_no_tool(),
                KeyCode::Digit1 => tools.set_tool(1),
                KeyCode::Digit2 => tools.set_tool(2),
                KeyCode::Digit3 => tools.set_tool(3),
                KeyCode::Digit4 => tools.set_tool(4),
                KeyCode::Digit5 => tools.set_tool(5),
                KeyCode::Digit6 => tools.set_tool(6),
                KeyCode::Digit7 => tools.set_tool(7),
                KeyCode::Digit8 => tools.set_tool(8),
                KeyCode::Digit9 => tools.set_tool(9),
                KeyCode::Digit0 => tools.set_tool(10),
                KeyCode::Space => tools.next_variant(),
                _ => (),
            }
        }
    }
}

fn setup_tools(mut commands: Commands) {
    let mut tools = Tools::default();

    tools.add(1, Box::new(ClearTool));
    tools.add(2, Box::new(ConveyorBeltTool::default()));
    tools.add(3, Box::new(GeneratorTool));
    tools.add(4, Box::new(SinkTool));
    tools.add(5, Box::new(DistributorTool));
    tools.add(6, Box::new(BridgeTool));

    commands.insert_resource(tools);
}

fn cleanup_tools(mut commands: Commands) {
    commands.remove_resource::<Tools>();
}

struct ClearTool;
impl Tool for ClearTool {
    fn get_sprite_flip(&self) -> (GameSprite, TileFlip) {
        (GameSprite::Delete, TileFlip::default())
    }

    fn creates_entity(&self) -> bool {
        false
    }

    fn configure_new_entity(&self, _: EntityCommands) {
        panic!();
    }
}

#[derive(Component)]
struct HoveredTile;

#[derive(Component)]
pub struct InteractionLayer;

fn make_interaction_layer(config: &MapConfig, texture: Handle<Image>) -> impl Bundle {
    (
        InteractionLayer,
        super::make_layer(config, texture, 1.0, "InteractionLayer"),
    )
}
