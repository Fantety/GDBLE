use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;

use async_trait::async_trait;
use btleplug::api::{
    Central as _, CentralEvent, CentralState, CharPropFlags, Characteristic, Manager as _,
    Peripheral as _, ScanFilter, WriteType,
};
use btleplug::platform::{Adapter, Manager, Peripheral};
use futures::{Stream, StreamExt};
use tokio::sync::RwLock;

use crate::ble_characteristic::{BleCharacteristicInfo, CharacteristicProperties};
use crate::ble_service::BleServiceInfo;
use crate::core::{DeviceId, GattKey};
use crate::types::{AdapterInfo, BleError, DeviceInfo};

pub type BackendEventStream =
    Pin<Box<dyn Stream<Item = Result<BackendEvent, BleError>> + Send + 'static>>;
pub type NotificationStream =
    Pin<Box<dyn Stream<Item = Result<BackendNotification, BleError>> + Send + 'static>>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AdapterState {
    Unknown,
    PoweredOff,
    PoweredOn,
}

impl AdapterState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::PoweredOff => "powered_off",
            Self::PoweredOn => "powered_on",
        }
    }
}

impl From<CentralState> for AdapterState {
    fn from(state: CentralState) -> Self {
        match state {
            CentralState::Unknown => Self::Unknown,
            CentralState::PoweredOff => Self::PoweredOff,
            CentralState::PoweredOn => Self::PoweredOn,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AdvertisementUpdate {
    Refresh,
    Rssi(i16),
    ManufacturerData(HashMap<u16, Vec<u8>>),
    ServiceData(HashMap<String, Vec<u8>>),
    Services(Vec<String>),
}

impl AdvertisementUpdate {
    pub fn apply(&self, info: &mut DeviceInfo) {
        match self {
            Self::Refresh => {}
            Self::Rssi(value) => info.rssi = Some(*value),
            Self::ManufacturerData(value) => info.manufacturer_data = value.clone(),
            Self::ServiceData(value) => info.service_data = value.clone(),
            Self::Services(value) => info.services = value.clone(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BackendEvent {
    // Full snapshots are injected only by the deterministic backend tests.
    #[cfg_attr(not(test), allow(dead_code))]
    Discovered(DeviceInfo),
    Disconnected(DeviceId),
    Changed {
        device: DeviceId,
        update: AdvertisementUpdate,
    },
    StateChanged(AdapterState),
    ServicesChanged(DeviceId),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackendNotification {
    pub characteristic_uuid: String,
    pub value: Vec<u8>,
}

#[async_trait]
pub trait BleBackend: Send + Sync {
    async fn initialize(&self) -> Result<(AdapterInfo, BackendEventStream), BleError>;
    async fn adapter_state(&self) -> Result<AdapterState, BleError>;
    async fn device_info(&self, device: &DeviceId) -> Result<DeviceInfo, BleError>;
    async fn is_connected(&self, device: &DeviceId) -> Result<bool, BleError>;
    async fn start_scan(&self) -> Result<(), BleError>;
    async fn stop_scan(&self) -> Result<(), BleError>;
    async fn connect(&self, device: &DeviceId) -> Result<(), BleError>;
    async fn disconnect(&self, device: &DeviceId) -> Result<(), BleError>;
    async fn discover_services(&self, device: &DeviceId) -> Result<Vec<BleServiceInfo>, BleError>;
    async fn read(&self, device: &DeviceId, key: &GattKey) -> Result<Vec<u8>, BleError>;
    async fn write(
        &self,
        device: &DeviceId,
        key: &GattKey,
        data: &[u8],
        with_response: bool,
    ) -> Result<(), BleError>;
    async fn subscribe(&self, device: &DeviceId, key: &GattKey) -> Result<(), BleError>;
    async fn unsubscribe(&self, device: &DeviceId, key: &GattKey) -> Result<(), BleError>;
    async fn notifications(&self, device: &DeviceId) -> Result<NotificationStream, BleError>;
}

#[derive(Clone, Default)]
pub struct BtleplugBackend {
    inner: Arc<BtleplugBackendInner>,
}

#[derive(Default)]
struct BtleplugBackendInner {
    adapter: RwLock<Option<Adapter>>,
    peripherals: RwLock<HashMap<DeviceId, Peripheral>>,
}

impl BtleplugBackend {
    async fn adapter(&self) -> Result<Adapter, BleError> {
        self.inner
            .adapter
            .read()
            .await
            .clone()
            .ok_or(BleError::NotInitialized)
    }

    async fn peripheral(&self, device: &DeviceId) -> Result<Peripheral, BleError> {
        if let Some(peripheral) = self.inner.peripherals.read().await.get(device).cloned() {
            return Ok(peripheral);
        }

        let adapter = self.adapter().await?;
        let peripherals = adapter
            .peripherals()
            .await
            .map_err(|error| BleError::OperationFailed(error.to_string()))?;
        for peripheral in peripherals {
            let id = DeviceId::parse(&peripheral.id().to_string())?;
            self.inner
                .peripherals
                .write()
                .await
                .insert(id.clone(), peripheral.clone());
            if &id == device {
                return Ok(peripheral);
            }
        }

        Err(BleError::DeviceNotFound(device.as_str().to_string()))
    }

    async fn snapshot(&self, peripheral: &Peripheral) -> Result<DeviceInfo, BleError> {
        let properties = peripheral
            .properties()
            .await
            .map_err(|error| BleError::OperationFailed(error.to_string()))?
            .ok_or_else(|| BleError::DeviceNotFound(peripheral.id().to_string()))?;
        let address = peripheral.id().to_string();
        let device_id = DeviceId::parse(&address)?;
        self.inner
            .peripherals
            .write()
            .await
            .insert(device_id, peripheral.clone());

        Ok(DeviceInfo::new(
            address,
            properties.local_name,
            properties.rssi,
            properties
                .services
                .iter()
                .map(|uuid| uuid.to_string().to_ascii_lowercase())
                .collect(),
            properties.manufacturer_data,
            properties
                .service_data
                .into_iter()
                .map(|(uuid, value)| (uuid.to_string().to_ascii_lowercase(), value))
                .collect(),
            properties.tx_power_level,
        ))
    }

    // Never await properties here: a slow snapshot must not hold up power or
    // connection events from the same adapter stream.
    fn map_event(event: CentralEvent) -> Option<Result<BackendEvent, BleError>> {
        let (id, update) = match event {
            CentralEvent::DeviceDiscovered(id) | CentralEvent::DeviceUpdated(id) => {
                (id, AdvertisementUpdate::Refresh)
            }
            CentralEvent::RssiUpdate { id, rssi } => (id, AdvertisementUpdate::Rssi(rssi)),
            CentralEvent::ManufacturerDataAdvertisement {
                id,
                manufacturer_data,
            } => (id, AdvertisementUpdate::ManufacturerData(manufacturer_data)),
            CentralEvent::ServiceDataAdvertisement { id, service_data } => (
                id,
                AdvertisementUpdate::ServiceData(
                    service_data
                        .into_iter()
                        .map(|(id, data)| (id.to_string(), data))
                        .collect(),
                ),
            ),
            CentralEvent::ServicesAdvertisement { id, services } => (
                id,
                AdvertisementUpdate::Services(
                    services.into_iter().map(|id| id.to_string()).collect(),
                ),
            ),
            CentralEvent::DeviceDisconnected(id) => {
                return Some(DeviceId::parse(&id.to_string()).map(BackendEvent::Disconnected))
            }
            CentralEvent::DeviceServicesModified(id) => {
                return Some(DeviceId::parse(&id.to_string()).map(BackendEvent::ServicesChanged))
            }
            CentralEvent::StateUpdate(state) => {
                return Some(Ok(BackendEvent::StateChanged(state.into())))
            }
            // Connect requests are completed by their backend future. Do not
            // create an extra public connected signal for its system echo.
            CentralEvent::DeviceConnected(_) => return None,
        };
        Some(
            DeviceId::parse(&id.to_string()).map(|device| BackendEvent::Changed { device, update }),
        )
    }

    async fn characteristic(
        &self,
        device: &DeviceId,
        key: &GattKey,
    ) -> Result<Characteristic, BleError> {
        let peripheral = self.peripheral(device).await?;
        peripheral
            .characteristics()
            .into_iter()
            .find(|characteristic| {
                characteristic
                    .uuid
                    .to_string()
                    .eq_ignore_ascii_case(key.characteristic_uuid())
                    && characteristic
                        .service_uuid
                        .to_string()
                        .eq_ignore_ascii_case(key.service_uuid())
            })
            .ok_or_else(|| {
                BleError::CharacteristicNotFound(format!(
                    "{} in service {}",
                    key.characteristic_uuid(),
                    key.service_uuid()
                ))
            })
    }
}

#[async_trait]
impl BleBackend for BtleplugBackend {
    async fn initialize(&self) -> Result<(AdapterInfo, BackendEventStream), BleError> {
        #[cfg(target_os = "android")]
        crate::android::ensure_initialized()?;

        let manager = Manager::new()
            .await
            .map_err(|error| BleError::InitializationFailed(error.to_string()))?;
        let mut adapters = manager
            .adapters()
            .await
            .map_err(|error| BleError::InitializationFailed(error.to_string()))?;
        let adapter = adapters.drain(..).next().ok_or(BleError::AdapterNotFound)?;
        let name = adapter
            .adapter_info()
            .await
            .map_err(|error| BleError::InitializationFailed(error.to_string()))?;
        let events = adapter
            .events()
            .await
            .map_err(|error| BleError::InitializationFailed(error.to_string()))?;
        self.inner.peripherals.write().await.clear();
        *self.inner.adapter.write().await = Some(adapter);

        let stream: BackendEventStream =
            Box::pin(events.filter_map(|event| async move { Self::map_event(event) }));
        #[cfg(target_os = "android")]
        let stream = crate::android::with_adapter_states(stream);

        Ok((AdapterInfo::new(name, None), stream))
    }

    async fn adapter_state(&self) -> Result<AdapterState, BleError> {
        #[cfg(target_os = "android")]
        {
            crate::android::adapter_state()
        }
        #[cfg(not(target_os = "android"))]
        self.adapter()
            .await?
            .adapter_state()
            .await
            .map(Into::into)
            .map_err(|error| BleError::OperationFailed(error.to_string()))
    }

    async fn device_info(&self, device: &DeviceId) -> Result<DeviceInfo, BleError> {
        self.snapshot(&self.peripheral(device).await?).await
    }

    async fn is_connected(&self, device: &DeviceId) -> Result<bool, BleError> {
        self.peripheral(device)
            .await?
            .is_connected()
            .await
            .map_err(|error| BleError::OperationFailed(error.to_string()))
    }

    async fn start_scan(&self) -> Result<(), BleError> {
        self.adapter()
            .await?
            .start_scan(ScanFilter::default())
            .await
            .map_err(|error| BleError::ScanFailed(error.to_string()))
    }

    async fn stop_scan(&self) -> Result<(), BleError> {
        self.adapter()
            .await?
            .stop_scan()
            .await
            .map_err(|error| BleError::ScanFailed(error.to_string()))
    }

    async fn connect(&self, device: &DeviceId) -> Result<(), BleError> {
        self.peripheral(device)
            .await?
            .connect()
            .await
            .map_err(|error| BleError::ConnectionFailed(error.to_string()))
    }

    async fn disconnect(&self, device: &DeviceId) -> Result<(), BleError> {
        self.peripheral(device)
            .await?
            .disconnect()
            .await
            .map_err(|error| BleError::OperationFailed(error.to_string()))
    }

    async fn discover_services(&self, device: &DeviceId) -> Result<Vec<BleServiceInfo>, BleError> {
        let peripheral = self.peripheral(device).await?;
        peripheral
            .discover_services()
            .await
            .map_err(|error| BleError::ServiceDiscoveryFailed(error.to_string()))?;
        let services = peripheral
            .services()
            .into_iter()
            .map(|service| {
                let characteristics = service
                    .characteristics
                    .iter()
                    .map(|characteristic| {
                        BleCharacteristicInfo::new(
                            characteristic.uuid.to_string().to_ascii_lowercase(),
                            CharacteristicProperties {
                                read: characteristic.properties.contains(CharPropFlags::READ),
                                write: characteristic.properties.contains(CharPropFlags::WRITE),
                                write_without_response: characteristic
                                    .properties
                                    .contains(CharPropFlags::WRITE_WITHOUT_RESPONSE),
                                notify: characteristic.properties.contains(CharPropFlags::NOTIFY),
                                indicate: characteristic
                                    .properties
                                    .contains(CharPropFlags::INDICATE),
                            },
                        )
                    })
                    .collect();
                BleServiceInfo::new(
                    service.uuid.to_string().to_ascii_lowercase(),
                    characteristics,
                )
            })
            .collect();
        Ok(services)
    }

    async fn read(&self, device: &DeviceId, key: &GattKey) -> Result<Vec<u8>, BleError> {
        let peripheral = self.peripheral(device).await?;
        let characteristic = self.characteristic(device, key).await?;
        peripheral
            .read(&characteristic)
            .await
            .map_err(|error| BleError::ReadFailed(error.to_string()))
    }

    async fn write(
        &self,
        device: &DeviceId,
        key: &GattKey,
        data: &[u8],
        with_response: bool,
    ) -> Result<(), BleError> {
        let peripheral = self.peripheral(device).await?;
        let characteristic = self.characteristic(device, key).await?;
        let write_type = if with_response {
            if !characteristic.properties.contains(CharPropFlags::WRITE) {
                return Err(BleError::WriteFailed(
                    "characteristic does not support writes with response".to_string(),
                ));
            }
            WriteType::WithResponse
        } else {
            if !characteristic
                .properties
                .contains(CharPropFlags::WRITE_WITHOUT_RESPONSE)
            {
                return Err(BleError::WriteFailed(
                    "characteristic does not support writes without response".to_string(),
                ));
            }
            WriteType::WithoutResponse
        };
        peripheral
            .write(&characteristic, data, write_type)
            .await
            .map_err(|error| BleError::WriteFailed(error.to_string()))
    }

    async fn subscribe(&self, device: &DeviceId, key: &GattKey) -> Result<(), BleError> {
        let peripheral = self.peripheral(device).await?;
        let characteristic = self.characteristic(device, key).await?;
        peripheral
            .subscribe(&characteristic)
            .await
            .map_err(|error| BleError::SubscribeFailed(error.to_string()))
    }

    async fn unsubscribe(&self, device: &DeviceId, key: &GattKey) -> Result<(), BleError> {
        let peripheral = self.peripheral(device).await?;
        let characteristic = self.characteristic(device, key).await?;
        peripheral
            .unsubscribe(&characteristic)
            .await
            .map_err(|error| BleError::UnsubscribeFailed(error.to_string()))
    }

    async fn notifications(&self, device: &DeviceId) -> Result<NotificationStream, BleError> {
        let stream = self
            .peripheral(device)
            .await?
            .notifications()
            .await
            .map_err(|error| BleError::SubscribeFailed(error.to_string()))?
            .map(|notification| {
                Ok(BackendNotification {
                    characteristic_uuid: notification.uuid.to_string().to_ascii_lowercase(),
                    value: notification.value,
                })
            });
        Ok(Box::pin(stream))
    }
}

#[cfg(any(test, feature = "test-harness"))]
#[cfg_attr(not(test), allow(dead_code))]
pub mod test_support;

#[cfg(all(test, any(target_os = "windows", target_os = "macos")))]
mod mapping_tests;
