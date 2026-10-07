use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;

use async_trait::async_trait;
use btleplug::api::{
    Central as _, CentralEvent, CharPropFlags, Characteristic, Manager as _, Peripheral as _,
    ScanFilter, WriteType,
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
pub enum BackendEvent {
    Discovered(DeviceInfo),
    Updated(DeviceInfo),
    Disconnected(DeviceId),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackendNotification {
    pub characteristic_uuid: String,
    pub value: Vec<u8>,
}

#[async_trait]
pub trait BleBackend: Send + Sync {
    async fn initialize(&self) -> Result<(AdapterInfo, BackendEventStream), BleError>;
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

    async fn map_event(&self, event: CentralEvent) -> Option<Result<BackendEvent, BleError>> {
        match event {
            CentralEvent::DeviceDiscovered(id) => {
                let adapter = match self.adapter().await {
                    Ok(adapter) => adapter,
                    Err(error) => return Some(Err(error)),
                };
                let peripheral = match adapter.peripheral(&id).await {
                    Ok(peripheral) => peripheral,
                    Err(error) => {
                        return Some(Err(BleError::DeviceNotFound(error.to_string())));
                    }
                };
                Some(
                    self.snapshot(&peripheral)
                        .await
                        .map(BackendEvent::Discovered),
                )
            }
            CentralEvent::DeviceUpdated(id) => {
                let adapter = match self.adapter().await {
                    Ok(adapter) => adapter,
                    Err(error) => return Some(Err(error)),
                };
                let peripheral = match adapter.peripheral(&id).await {
                    Ok(peripheral) => peripheral,
                    Err(error) => {
                        return Some(Err(BleError::DeviceNotFound(error.to_string())));
                    }
                };
                Some(self.snapshot(&peripheral).await.map(BackendEvent::Updated))
            }
            CentralEvent::DeviceDisconnected(id) => {
                Some(DeviceId::parse(&id.to_string()).map(BackendEvent::Disconnected))
            }
            _ => None,
        }
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

        let backend = self.clone();
        let stream = events
            .then(move |event| {
                let backend = backend.clone();
                async move { backend.map_event(event).await }
            })
            .filter_map(|event| async move { event });

        Ok((AdapterInfo::new(name, None), Box::pin(stream)))
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

#[cfg(test)]
pub mod test_support;
