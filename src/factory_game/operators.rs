use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;

use crate::{
    GameState,
    factory_game::{
        BaseLayer, ConveyorSystems,
        conveyor::Conveyor,
        helpers::ConveyorDirection,
        interaction::{PlaceTileEvent, RegisterPlaceTileEvent, Tool},
        payloads::{PayloadTransferredEvent, RequestPayloadTransferEvent},
    },
    sprite_sheet::GameSprite,
};

pub fn operators_plugin(app: &mut App) {
    app.register_place_tile_event::<PlaceOperatorEvent>()
        .register_type::<Operator>()
        .register_type::<OperatorTile>()
        .register_type::<Operand>()
        .add_systems(
            Update,
            (
                update_operator_tiles.in_set(ConveyorSystems::TileUpdater),
                (transfer_payloads_to_operators, generate_new_payloads)
                    .chain()
                    .in_set(ConveyorSystems::TransportLogic),
            ),
        );
}

#[derive(Debug, Clone, Copy, Reflect)]
enum Operator {
    Plus,
    Multiply,
}

impl Operator {
    fn sprite(&self) -> GameSprite {
        match self {
            Operator::Plus => GameSprite::OperatorPlus,
            Operator::Multiply => GameSprite::OperatorMultiply,
        }
    }

    fn generate_operand(&self, left: Operand, right: Operand) -> Operand {
        match self {
            Operator::Plus => Operand(left.0.checked_add(right.0).unwrap_or(1)),
            Operator::Multiply => Operand(left.0.checked_mul(right.0).unwrap_or(1)),
        }
    }
}

pub struct OperatorsTool {
    operator: Operator,
    direction: ConveyorDirection,
}

impl OperatorsTool {
    fn new(operator: Operator) -> Self {
        Self {
            operator,
            direction: ConveyorDirection::North,
        }
    }
    pub fn plus() -> Self {
        Self::new(Operator::Plus)
    }

    pub fn multiply() -> Self {
        Self::new(Operator::Multiply)
    }
}

impl Tool for OperatorsTool {
    fn get_sprite_flip(&self) -> (GameSprite, TileFlip) {
        (self.operator.sprite(), self.direction.tile_flip())
    }

    fn next_variant(&mut self) {
        self.direction = self.direction.next();
    }

    fn execute(&self, mut commands: Commands, tile_pos: &TilePos) {
        commands.trigger(PlaceOperatorEvent(*tile_pos, self.operator, self.direction));
    }
}

#[derive(Event, Debug)]
pub struct PlaceOperatorEvent(TilePos, Operator, ConveyorDirection);

impl PlaceTileEvent for PlaceOperatorEvent {
    fn tile_pos(&self) -> TilePos {
        self.0
    }

    fn configure_new_entity(&self, mut commands: EntityCommands) {
        commands.insert((
            Name::new(format!("{:?}", self.1)),
            operator_bundle(self.1, self.2),
        ));
    }
}

#[derive(Component, Debug, Reflect, Clone, Copy)]
pub struct Operand(pub u32);

impl Operand {
    fn payload_text(&self) -> String {
        format!("{}", self.0)
    }
}

#[derive(Component, Debug, Reflect)]
struct OperatorTile {
    operator: Operator,
    left_operand: Option<(Entity, Operand)>,
    right_operand: Option<(Entity, Operand)>,
}

impl OperatorTile {
    pub fn new(operator: Operator) -> Self {
        OperatorTile {
            operator,
            left_operand: None,
            right_operand: None,
        }
    }

    pub fn sprite(&self) -> GameSprite {
        self.operator.sprite()
    }
}

fn operator_bundle(operator: Operator, direction: ConveyorDirection) -> impl Bundle {
    (OperatorTile::new(operator), Conveyor::from(direction))
}

fn update_operator_tiles(
    mut commands: Commands,
    new_operators: Query<(Entity, &OperatorTile, &Conveyor), Without<TileTextureIndex>>,
    tilemap_entity: Single<Entity, (With<BaseLayer>, With<TileStorage>)>,
) {
    for (entity, operator, conveyor) in new_operators {
        commands.entity(entity).insert_if_new(TileBundle {
            tilemap_id: TilemapId(*tilemap_entity),
            texture_index: operator.sprite().tile_texture_index(),
            flip: conveyor.output().tile_flip(),
            ..default()
        });
    }
}

fn transfer_payloads_to_operators(
    mut transfers: EventReader<RequestPayloadTransferEvent>,
    mut operators: Query<(&Conveyor, &mut OperatorTile)>,
    operands: Query<&Operand>,
) {
    for RequestPayloadTransferEvent {
        payload,
        destination,
        direction,
        ..
    } in transfers.read()
    {
        if let Ok((conveyor, mut operator)) = operators.get_mut(*destination)
            && let Ok(operand) = operands.get(*payload)
        {
            let incoming_direction = direction.opposite();

            if incoming_direction == conveyor.output().left() && operator.left_operand.is_none() {
                operator.left_operand = Some((*payload, *operand));
            } else if incoming_direction == conveyor.output().right()
                && operator.right_operand.is_none()
            {
                operator.right_operand = Some((*payload, *operand));
            }

            // NOTE: the transfer only actually happens in
            // generate_new_payloads. This isn't ideal and should be changed.
        }
    }
}

fn generate_new_payloads() {}

pub fn operand_bundle(operand: Operand) -> impl Bundle {
    (
        StateScoped(GameState::FactoryGame),
        Name::new(format!("Payload {}", operand.payload_text())),
        operand,
        Text2d::new(operand.payload_text()),
        TextColor(Color::srgb(1.0, 0.4, 0.4)),
    )
}
