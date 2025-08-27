use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;
use smallvec::SmallVec;

use crate::{
    factory_game::{
        BaseLayer, ConveyorSystems,
        conveyor::{Conveyor, TilesToCheck},
        conveyor_belts::find_incoming_directions,
        helpers::{ConveyorDirection, ConveyorDirections, get_neighbors_from_query},
        interaction::{PlaceTileEvent, RegisterPlaceTileEvent, Tool},
        payloads::{
            PayloadMarker, PayloadTransferredEvent, RequestPayloadTransferEvent,
            get_payload_transform,
        },
    },
    helpers::TilemapQuery,
    sprite_sheet::GameSprite,
};

pub fn distributor_plugin(app: &mut App) {
    app.register_place_tile_event::<PlaceDistributorEvent>()
        .register_type::<DistributorConveyor>()
        .add_systems(
            Update,
            (
                transfer_payloads_to_distributors.in_set(ConveyorSystems::TransferPayloads),
                transfer_payloads_from_distributors.in_set(ConveyorSystems::TransferredPayloads),
                update_distributor_payloads.in_set(ConveyorSystems::TransportLogic),
                (update_distributor_conveyors, update_distributor_tiles)
                    .in_set(ConveyorSystems::TileUpdater),
                update_distributor_payload_transforms.in_set(ConveyorSystems::PayloadTransforms),
            ),
        )
        .add_observer(on_remove_distributor_conveyor);
}

pub struct DistributorTool;
impl Tool for DistributorTool {
    fn get_sprite_flip(&self) -> (GameSprite, TileFlip) {
        (GameSprite::Distributor, TileFlip::default())
    }

    fn execute(&self, mut commands: Commands, tile_pos: &TilePos) {
        commands.trigger(PlaceDistributorEvent(*tile_pos));
    }
}

#[derive(Event, Debug)]
pub struct PlaceDistributorEvent(TilePos);

impl PlaceTileEvent for PlaceDistributorEvent {
    fn tile_pos(&self) -> TilePos {
        self.0
    }

    fn configure_new_entity(&self, mut commands: EntityCommands) {
        commands.insert((Distributor, Name::new("Distributor")));
    }
}

#[derive(Component)]
#[require(Conveyor::new(ConveyorDirections::all()), DistributorConveyor::new(5))]
struct Distributor;

#[derive(Component, Debug, Reflect)]
pub struct DistributorConveyor {
    next_output: ConveyorDirection,
    payloads: SmallVec<[DistributedPayload; 2]>,
    capacity: u32,
}

impl DistributorConveyor {
    pub fn new(capacity: u32) -> Self {
        Self {
            next_output: ConveyorDirection::default(),
            payloads: SmallVec::default(),
            capacity,
        }
    }

    fn spacing(&self) -> f32 {
        1.0 / (self.capacity as f32)
    }

    #[expect(clippy::too_many_arguments)]
    pub fn try_take<F>(
        &mut self,
        conveyor: &Conveyor,
        tile_storage: &TileStorage,
        tile_pos: &TilePos,
        map_size: &TilemapSize,
        conveyors: &Query<&Conveyor>,
        from: Option<ConveyorDirection>,
        get_entity_to_take: F,
    ) -> bool
    where
        F: FnOnce() -> Entity,
    {
        // We can only take a payload if there's room for it
        if self.payloads.len() >= self.capacity as usize {
            return false;
        }

        if !self.payloads.is_empty() && self.payloads[0].mu < self.spacing() {
            return false;
        }

        // We can only take a payload if there's a destination for it
        let neighbors = get_neighbors_from_query(tile_storage, tile_pos, map_size, conveyors);
        let destination_direction =
            conveyor
                .outputs()
                .iter_from(self.next_output)
                .find(|direction| {
                    let neighbor = neighbors.get((*direction).into());
                    neighbor
                        .map(|conveyor| conveyor.inputs().is_set(direction.opposite()))
                        .unwrap_or(false)
                });
        if let Some(destination_direction) = destination_direction {
            self.payloads.push(DistributedPayload {
                entity: get_entity_to_take(),
                to: destination_direction,
                from,
                mu: 0.0,
            });
            self.next_output = destination_direction.next();
            return true;
        }
        false
    }

    fn update_payloads(&mut self, t: f32) {
        assert!(
            self.payloads.iter().all(|p| p.mu >= 0.0 && p.mu <= 1.0),
            "All payload mu's in rage 0 <= mu <= 1: {:?}",
            self.payloads
        );
        assert!(
            self.payloads.is_sorted_by_key(|p| -p.mu),
            "Payloads are sorted by descending mu: {:?}.",
            self.payloads
        );
        let spacing = self.spacing();

        let mut last_mu = None;
        for p in self.payloads.iter_mut() {
            let max_mu: f32 = last_mu.map(|mu| mu - spacing).unwrap_or(1.0);
            p.mu = max_mu.min(p.mu + t);
            last_mu = Some(p.mu);
        }
    }

    fn get_payload_to_transfer(&self) -> Option<&DistributedPayload> {
        if let Some(p) = self.payloads.first()
            && p.mu == 1.0
        {
            return Some(p);
        }

        None
    }

    fn remove_payload(&mut self, payload: Entity) {
        self.payloads.retain(|p| p.entity != payload);
    }

    fn despawn_payloads(&self, mut commands: Commands) {
        self.payloads
            .iter()
            .for_each(|p| commands.entity(p.entity).despawn());
    }
}

fn on_remove_distributor_conveyor(
    trigger: Trigger<OnRemove, DistributorConveyor>,
    distributors: Query<&DistributorConveyor>,
    commands: Commands,
) {
    if let Ok(distributor) = distributors.get(trigger.target()) {
        distributor.despawn_payloads(commands);
    }
}

#[derive(Debug, Reflect)]
struct DistributedPayload {
    entity: Entity,
    from: Option<ConveyorDirection>,
    to: ConveyorDirection,
    mu: f32,
}

fn update_distributor_conveyors(
    to_check: Res<TilesToCheck>,
    mut conveyors: Query<&mut Conveyor>,
    distributors: Query<(), With<Distributor>>,
    base: Single<(&TileStorage, &TilemapSize), With<BaseLayer>>,
) {
    let (tile_storage, map_size) = base.into_inner();

    for tile_pos in &to_check.0 {
        if let Some(entity) = tile_storage.get(tile_pos)
            && distributors.contains(entity)
        {
            let inputs = find_incoming_directions(
                tile_pos,
                tile_storage,
                map_size,
                &conveyors.as_readonly(),
            );

            if let Ok(mut conveyor) = conveyors.get_mut(entity) {
                conveyor.set_inputs(inputs);
                conveyor.set_outputs(ConveyorDirections::all_except(inputs));
            }
        }
    }
}

fn transfer_payloads_to_distributors(
    mut transfers: EventReader<RequestPayloadTransferEvent>,
    mut receivers: Query<(&TilePos, &Conveyor, &mut DistributorConveyor), With<Distributor>>,
    mut transferred: EventWriter<PayloadTransferredEvent>,
    base: Single<(&TileStorage, &TilemapSize), With<BaseLayer>>,
    conveyors: Query<&Conveyor>,
) {
    let (tile_storage, map_size) = base.into_inner();

    for e in transfers.read() {
        if let Ok((tile_pos, conveyor, mut distributor)) = receivers.get_mut(e.destination)
            && distributor.try_take(
                conveyor,
                tile_storage,
                tile_pos,
                map_size,
                &conveyors,
                Some(e.direction.opposite()),
                || e.payload,
            )
        {
            transferred.write(PayloadTransferredEvent {
                payload: e.payload,
                source: e.source,
            });
        }
    }
}

fn transfer_payloads_from_distributors(
    mut transferred: EventReader<PayloadTransferredEvent>,
    mut distributors: Query<&mut DistributorConveyor>,
) {
    for e in transferred.read() {
        if let Ok(mut distributor) = distributors.get_mut(e.source) {
            distributor.remove_payload(e.payload);
        }
    }
}

fn update_distributor_payloads(
    distributors: Query<(Entity, &mut DistributorConveyor, &TilePos)>,
    time: Res<Time>,
    base: Single<(&TileStorage, &TilemapSize), With<BaseLayer>>,
    mut send_payloads: EventWriter<RequestPayloadTransferEvent>,
) {
    let (tile_storage, map_size) = base.into_inner();
    let t = time.delta_secs();

    for (source, mut distributor, tile_pos) in distributors {
        distributor.update_payloads(t);
        if let Some(payload) = distributor.get_payload_to_transfer() {
            let destination_pos = tile_pos.square_offset(&payload.to.into(), map_size);
            let destination_entity = destination_pos.and_then(|pos| tile_storage.get(&pos));
            if let Some(destination) = destination_entity {
                let e = RequestPayloadTransferEvent {
                    payload: payload.entity,
                    source,
                    destination,
                    direction: payload.to,
                };
                send_payloads.write(e);
            }
        }
    }
}

fn update_distributor_tiles(
    mut commands: Commands,
    new_distributors: Query<Entity, Added<Distributor>>,
    tilemap_entity: Single<Entity, (With<BaseLayer>, With<TilemapSize>)>,
) {
    for e in new_distributors {
        commands.entity(e).insert_if_new(TileBundle {
            tilemap_id: TilemapId(*tilemap_entity),
            texture_index: GameSprite::Distributor.tile_texture_index(),
            ..default()
        });
    }
}

fn update_distributor_payload_transforms(
    distributors: Query<(&TilePos, &DistributorConveyor)>,
    mut payloads: Query<&mut Transform, With<PayloadMarker>>,
    base: Single<TilemapQuery, With<BaseLayer>>,
) {
    for (tile_pos, distributor) in distributors {
        let tile_center = base.center_in_world(tile_pos);
        for p in &distributor.payloads {
            if let Ok(mut transform) = payloads.get_mut(p.entity) {
                *transform =
                    get_payload_transform(tile_center, base.tile_size, p.from, Some(p.to), p.mu);
            }
        }
    }
}
