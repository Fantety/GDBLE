use super::*;

#[test]
fn all_advertisement_categories_and_control_events_are_mapped_independently() {
    #[cfg(target_os = "windows")]
    let id: btleplug::platform::PeripheralId = "AA:BB:CC:DD:EE:FF"
        .parse::<btleplug::api::BDAddr>()
        .unwrap()
        .into();
    #[cfg(target_os = "macos")]
    let id: btleplug::platform::PeripheralId = uuid::Uuid::nil().into();
    let service = uuid::Uuid::from_u128(0x180d);
    let events = [
        (
            CentralEvent::RssiUpdate {
                id: id.clone(),
                rssi: -41,
            },
            AdvertisementUpdate::Rssi(-41),
        ),
        (
            CentralEvent::ManufacturerDataAdvertisement {
                id: id.clone(),
                manufacturer_data: HashMap::from([(7, vec![1])]),
            },
            AdvertisementUpdate::ManufacturerData(HashMap::from([(7, vec![1])])),
        ),
        (
            CentralEvent::ServiceDataAdvertisement {
                id: id.clone(),
                service_data: HashMap::from([(service, vec![2])]),
            },
            AdvertisementUpdate::ServiceData(HashMap::from([(service.to_string(), vec![2])])),
        ),
        (
            CentralEvent::ServicesAdvertisement {
                id: id.clone(),
                services: vec![service],
            },
            AdvertisementUpdate::Services(vec![service.to_string()]),
        ),
    ];
    for (event, expected) in events {
        assert!(
            matches!(BtleplugBackend::map_event(event), Some(Ok(BackendEvent::Changed { update, .. })) if update == expected)
        );
    }
    assert!(matches!(
        BtleplugBackend::map_event(CentralEvent::DeviceServicesModified(id)),
        Some(Ok(BackendEvent::ServicesChanged(_)))
    ));
    for state in [
        CentralState::Unknown,
        CentralState::PoweredOff,
        CentralState::PoweredOn,
    ] {
        assert!(matches!(
            BtleplugBackend::map_event(CentralEvent::StateUpdate(state)),
            Some(Ok(BackendEvent::StateChanged(_)))
        ));
    }
}
