use bevy::{ecs::component::Mutable, prelude::*, reflect::GetTypeRegistration};

use crate::factory_game::{
    ConveyorSystems,
    conveyor::Conveyor,
    payloads::{PayloadTransferredEvent, RequestPayloadTransferEvent},
};

pub trait PayloadHandler: GetTypeRegistration + Component<Mutability = Mutable> {
    fn try_transfer(
        &mut self,
        self_conveyor: &Conveyor,
        request: &RequestPayloadTransferEvent,
    ) -> Option<Entity>;

    fn remove_payload(&mut self, payload: Entity);

    fn iter_payloads(&self) -> impl Iterator<Item = Entity>;
}

pub trait AddPayloadHandler {
    fn add_payload_handler<T: PayloadHandler>(&mut self) -> &mut Self;
}

impl AddPayloadHandler for App {
    fn add_payload_handler<T: PayloadHandler>(&mut self) -> &mut Self {
        self.register_type::<T>()
            .add_systems(
                Update,
                (
                    transfer_payloads_to_handlers::<T>
                        .in_set(ConveyorSystems::TransferPayloadsToHandlers),
                    transfer_payloads_from_handlers::<T>
                        .in_set(ConveyorSystems::TransferPayloadsFromHandlers),
                ),
            )
            .add_observer(on_remove_handler::<T>)
    }
}

fn transfer_payloads_to_handlers<T: PayloadHandler>(
    mut transfers: EventReader<RequestPayloadTransferEvent>,
    mut handlers: Query<(&Conveyor, &mut T)>,
    mut transferred: EventWriter<PayloadTransferredEvent>,
) {
    for e in transfers.read() {
        if let Ok((conveyor, mut handler)) = handlers.get_mut(e.destination)
            && handler.try_transfer(conveyor, e).is_some()
        {
            transferred.write(PayloadTransferredEvent {
                payload: e.payload,
                source: e.source,
            });
        }
    }
}

fn transfer_payloads_from_handlers<T: PayloadHandler>(
    mut transferred: EventReader<PayloadTransferredEvent>,
    mut handlers: Query<&mut T>,
) {
    for e in transferred.read() {
        if let Ok(mut handler) = handlers.get_mut(e.source) {
            handler.remove_payload(e.payload);
        }
    }
}

fn on_remove_handler<T: PayloadHandler>(
    trigger: Trigger<OnRemove, T>,
    handlers: Query<&T>,
    mut commands: Commands,
) {
    if let Ok(handler) = handlers.get(trigger.target()) {
        handler
            .iter_payloads()
            .for_each(|payload| commands.entity(payload).despawn());
    }
}
