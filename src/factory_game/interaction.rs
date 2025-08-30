use bevy::{
    input::{ButtonState, common_conditions::input_just_pressed, keyboard::KeyboardInput},
    prelude::*,
};
use bevy_ecs_tilemap::prelude::*;
use bevy_egui::input::{EguiWantsInput, egui_wants_any_input};
use bevy_pancam::PanCam;
use std::fmt::Debug;

use crate::{
    GameState,
    factory_game::{
        BaseLayer, ConveyorSystems, MapConfig, bridge::BridgeTool, conveyor::ConveyorUpdated,
        conveyor_belts::ConveyorBeltTool, distributor::DistributorTool, generator::GeneratorTool,
        operators::OperatorsTool, sink::SinkTool,
    },
    helpers::TilemapQuery,
    sprite_sheet::{GameSprite, SpriteSheet},
};

pub fn interaction_plugin(app: &mut App) {
    app.register_type::<Tools>()
        .register_place_tile_event::<ClearTileEvent>()
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
                (
                    update_hovered_tile.run_if(resource_exists_and_changed::<Tools>),
                    flash_hovered_tile,
                )
                    .in_set(ConveyorSystems::TileUpdater),
                give_control_to_egui
                    .run_if(in_state(GameState::FactoryGame))
                    .run_if(
                        resource_exists_and_changed::<Tools>
                            .or(resource_exists_and_changed::<EguiWantsInput>),
                    ),
            ),
        );
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

fn flash_hovered_tile(q: Option<Single<&mut TileColor, With<HoveredTile>>>, time: Res<Time>) {
    if let Some(mut color) = q {
        let bright_pulse = 1.0 + ((time.elapsed_secs() * 5.0).sin() + 1.0) / 2.0;
        let alpha_pulse = ((time.elapsed_secs() * 10.0).sin() + 1.0) / 2.0;
        **color = TileColor(Color::hsla(0.0, 0.5, bright_pulse, alpha_pulse));
    }
}

fn track_mouse(
    mut cursor_moved: EventReader<CursorMoved>,
    camera_query: Single<(&GlobalTransform, &Camera)>,
    interaction_layer: Single<TilemapQuery, With<InteractionLayer>>,
    mut hovered_tile: Single<&mut TilePos, With<HoveredTile>>,
) {
    if let Some(e) = cursor_moved.read().last() {
        let (global_transform, camera) = *camera_query;
        if let Ok(p) = camera.viewport_to_world_2d(global_transform, e.position)
            && let Some(tile_pos) = interaction_layer.get_tile_pos_from_world_pos(&p)
        {
            **hovered_tile = tile_pos;
        }
    }
}

fn on_click(commands: Commands, tile_pos: Single<&TilePos, With<HoveredTile>>, tools: Res<Tools>) {
    if let Some(tool) = tools.current_tool() {
        tool.tool.execute(commands, *tile_pos);
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

    pub fn current_tool(&self) -> Option<&ToolEntry> {
        self.current_tool.map(|index| &self.tools[index])
    }

    pub fn tools(&self) -> &Vec<ToolEntry> {
        &self.tools
    }
}

impl ToolEntry {
    pub fn slot(&self) -> u32 {
        self.slot
    }

    pub fn tool(&self) -> &dyn Tool {
        self.tool.as_ref()
    }
}

pub trait Tool: Sync + Send {
    fn get_sprite_flip(&self) -> (GameSprite, TileFlip);
    fn next_variant(&mut self) {}

    fn execute(&self, commands: Commands, tile_pos: &TilePos);
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
    tools.add(7, Box::new(OperatorsTool::plus()));
    tools.add(8, Box::new(OperatorsTool::multiply()));

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

    fn execute(&self, mut commands: Commands, tile_pos: &TilePos) {
        commands.trigger(ClearTileEvent(*tile_pos));
    }
}

pub trait PlaceTileEvent<T = ()>: Event
where
    T: Debug,
{
    fn tile_pos(&self) -> TilePos;

    fn make_new_entity(&self, mut commands: Commands, storage: &mut TileStorage) -> Option<Entity> {
        let entity = commands
            .spawn((
                StateScoped(GameState::FactoryGame),
                BaseLayer,
                self.tile_pos(),
            ))
            .id();

        storage.set(&self.tile_pos(), entity);

        Some(entity)
    }

    #[expect(unused_variables)]
    fn configure_new_entity(&self, commands: EntityCommands) {}
}

fn handle_place_tile_event<T: PlaceTileEvent + Debug>(
    trigger: Trigger<T>,
    mut commands: Commands,
    mut storage: Single<&mut TileStorage, With<BaseLayer>>,
    mut despawned_event: EventWriter<ConveyorUpdated>,
) {
    let tile_pos = trigger.tile_pos();

    if let Some(entity) = storage.remove(&tile_pos) {
        commands.entity(entity).despawn();
        despawned_event.write(ConveyorUpdated(tile_pos));
    }

    if let Some(entity) = trigger.make_new_entity(commands.reborrow(), &mut storage) {
        trigger.configure_new_entity(commands.entity(entity));
    }
}

pub trait RegisterPlaceTileEvent {
    fn register_place_tile_event<T: PlaceTileEvent + Debug>(&mut self) -> &mut Self;
}

impl RegisterPlaceTileEvent for App {
    fn register_place_tile_event<T: PlaceTileEvent + Debug>(&mut self) -> &mut Self {
        self.add_event::<T>()
            .add_observer(handle_place_tile_event::<T>)
    }
}

#[derive(Event, Debug)]
pub struct ClearTileEvent(TilePos);

impl PlaceTileEvent for ClearTileEvent {
    fn tile_pos(&self) -> TilePos {
        self.0
    }
    fn make_new_entity(&self, _: Commands, _: &mut TileStorage) -> Option<Entity> {
        None
    }
}

#[derive(Component)]
struct HoveredTile;

#[derive(Component)]
pub struct InteractionLayer;

fn make_interaction_layer(config: &MapConfig, texture: Handle<Image>) -> impl Bundle {
    (
        InteractionLayer,
        super::make_layer(config, texture, 10.0, "InteractionLayer"),
    )
}
