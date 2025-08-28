use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;

use crate::{
    GameState,
    factory_game::{
        BaseLayer, ConveyorSystems,
        conveyor::Conveyor,
        helpers::ConveyorDirection,
        interaction::{PlaceTileEvent, RegisterPlaceTileEvent, Tool},
        payload_handler::{AddPayloadHandler, PayloadHandler},
        payloads::{
            Payload, PayloadTransportLine, RequestPayloadTransferEvent, get_payload_transform,
        },
    },
    helpers::{TilemapQuery, TilemapQueryItem},
    sprite_sheet::GameSprite,
};

pub fn operators_plugin(app: &mut App) {
    app.register_place_tile_event::<PlaceOperatorEvent>()
        .add_payload_handler::<OperatorTile>()
        .register_type::<Operator>()
        .register_type::<Operand>()
        .add_systems(
            Update,
            (
                update_operator_tiles.in_set(ConveyorSystems::TileUpdater),
                (generate_new_payloads, update_operator_payloads)
                    .in_set(ConveyorSystems::TransportLogic),
                update_operator_payload_transforms.in_set(ConveyorSystems::PayloadTransforms),
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

    fn generate_operand(&self, left: &Operand, right: &Operand) -> Operand {
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
    left_operand: Option<Entity>,
    right_operand: Option<Entity>,
    payload_transport_line: PayloadTransportLine,
}

impl PayloadHandler for OperatorTile {
    fn try_transfer(
        &mut self,
        self_conveyor: &Conveyor,
        request: &RequestPayloadTransferEvent,
    ) -> bool {
        let incoming_direction = request.direction.opposite();

        if incoming_direction == self_conveyor.output().left() && self.left_operand.is_none() {
            self.left_operand = Some(request.payload);
            return true;
        } else if incoming_direction == self_conveyor.output().right()
            && self.right_operand.is_none()
        {
            self.right_operand = Some(request.payload);
            return true;
        }
        false
    }

    fn remove_payload(&mut self, payload: Entity) {
        self.payload_transport_line.remove_payload(payload);
    }

    fn iter_payloads(&self) -> impl Iterator<Item = Entity> {
        self.payload_transport_line
            .iter_payloads()
            .chain(self.left_operand)
            .chain(self.right_operand)
    }
}

impl OperatorTile {
    pub fn new(operator: Operator, direction: ConveyorDirection) -> Self {
        OperatorTile {
            operator,
            left_operand: None,
            right_operand: None,
            payload_transport_line: PayloadTransportLine::new(direction, 2),
        }
    }

    pub fn sprite(&self) -> GameSprite {
        self.operator.sprite()
    }
}

fn update_operator_payloads(
    operators: Query<(Entity, &mut OperatorTile, &TilePos)>,
    time: Res<Time>,
    base: Single<(&TileStorage, &TilemapSize), With<BaseLayer>>,
    mut send_payloads: EventWriter<RequestPayloadTransferEvent>,
) {
    let (tile_storage, map_size) = base.into_inner();
    let t = time.delta_secs();

    for (entity, mut operator, tile_pos) in operators {
        operator.payload_transport_line.update(
            entity,
            tile_pos,
            t,
            tile_storage,
            map_size,
            &mut send_payloads,
        );
    }
}

fn generate_new_payloads(
    mut commands: Commands,
    operators: Query<&mut OperatorTile>,
    operands: Query<&Operand>,
) {
    for mut operator in operators {
        if let Some(left_entity) = operator.left_operand
            && let Some(right_entity) = operator.right_operand
            && let Ok(left_operand) = operands.get(left_entity)
            && let Ok(right_operand) = operands.get(right_entity)
        {
            let output_direction = operator.payload_transport_line.output_direction();
            let new_operand = operator
                .operator
                .generate_operand(left_operand, right_operand);

            if operator.payload_transport_line.try_transfer_onto_with_mu(
                output_direction.opposite(),
                0.5,
                || commands.spawn(operand_bundle(new_operand)).id(),
            ) {
                commands.entity(left_entity).try_despawn();
                commands.entity(right_entity).try_despawn();
                operator.left_operand = None;
                operator.right_operand = None;
            }
        }
    }
}

fn update_operator_payload_transforms(
    operators: Query<(&TilePos, &mut OperatorTile, &Conveyor)>,
    mut payloads: Query<&mut Transform, With<Payload>>,
    base: Single<TilemapQuery, With<BaseLayer>>,
) {
    for (tile_pos, operator, conveyor) in operators {
        operator
            .payload_transport_line
            .update_payload_transforms(tile_pos, &mut payloads, &base);

        if let Some(entity) = operator.left_operand
            && let Ok(mut transform) = payloads.get_mut(entity)
        {
            *transform = get_operand_transform(&base, tile_pos, conveyor.output().left());
        }
        if let Some(entity) = operator.right_operand
            && let Ok(mut transform) = payloads.get_mut(entity)
        {
            *transform = get_operand_transform(&base, tile_pos, conveyor.output().right());
        }
    }
}

fn get_operand_transform(
    base: &TilemapQueryItem,
    tile_pos: &TilePos,
    direction: ConveyorDirection,
) -> Transform {
    let tile_center = base.center_in_world(tile_pos);
    let transform = get_payload_transform(tile_center, base.tile_size, None, Some(direction), 1.0);

    let scale_center = tile_center.extend(0.0) - transform.translation;

    transform
        * Transform::from_translation(scale_center)
        * Transform::from_scale(Vec3::splat(0.75))
        * Transform::from_translation(-scale_center)
}

fn operator_bundle(operator: Operator, direction: ConveyorDirection) -> impl Bundle {
    (
        OperatorTile::new(operator, direction),
        Conveyor::from(direction),
    )
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

pub fn operand_bundle(operand: Operand) -> impl Bundle {
    (
        StateScoped(GameState::FactoryGame),
        Name::new(format!("Payload {}", operand.payload_text())),
        operand,
        Payload,
        Text2d::new(operand.payload_text()),
        TextColor(Color::srgb(1.0, 0.4, 0.4)),
    )
}
