use super::*;

#[derive(Event, Debug)]
pub struct RequestPayloadTransferEvent {
    pub payload: Entity,
    pub destination: Entity,
    pub direction: ConveyorDirection,
}

#[derive(Component, Default)]
pub struct SimpleConveyorTransferPolicy;

pub fn transfer_payloads_standard(
    mut commands: Commands,
    mut transfers: EventReader<RequestPayloadTransferEvent>,
    receivers: Query<(&Conveyor, Option<&Payloads>), With<SimpleConveyorTransferPolicy>>,
) {
    for RequestPayloadTransferEvent {
        payload,
        destination,
        direction,
    } in transfers.read()
    {
        if let Ok((conveyor, payloads)) = receivers.get(*destination) {
            if conveyor.inputs.is_none() {
                continue;
            }
            const MAX_PAYLOADS: usize = 1;

            let current_payload_count = payloads.map(|p| p.len()).unwrap_or(0);

            if current_payload_count < MAX_PAYLOADS {
                commands.entity(*payload).insert((
                    Payload(*destination),
                    PayloadTransport {
                        source: Some(direction.opposite()),
                        destination: conveyor.single_or_no_output(),
                        ..default()
                    },
                ));
            }
        }
    }
}
