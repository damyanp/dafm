use super::*;

#[derive(Event, Debug)]
pub struct RequestPayloadTransferEvent {
    pub payload: Entity,
    pub destination: Entity,
}

#[derive(Component, Default)]
pub struct SimpleConveyorTransferPolicy;

pub fn transfer_payloads_standard(
    mut commands: Commands,
    mut transfers: EventReader<RequestPayloadTransferEvent>,
    receivers: Query<(&Conveyor, Option<&Payloads>), With<SimpleConveyorTransferPolicy>>,
    payload_destinations: Query<&PayloadDestination>,
) {
    for RequestPayloadTransferEvent {
        payload,
        destination,
    } in transfers.read()
    {
        if let Ok((conveyor, payloads)) = receivers.get(*destination) {
            if conveyor.inputs.is_none() {
                continue;
            }
            const MAX_PAYLOADS: usize = 1;

            let current_payload_count = payloads.map(|p| p.len()).unwrap_or(0);

            if current_payload_count < MAX_PAYLOADS {
                take_payload(
                    commands.reborrow(),
                    *payload,
                    *destination,
                    payload_destinations.get(*payload).ok(),
                );
            }
        }
    }
}
