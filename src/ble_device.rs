use btleplug::api::{CharPropFlags, Characteristic, Peripheral as _, WriteType};
use btleplug::platform::Peripheral;
use godot::prelude::*;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use tokio::runtime::Runtime;

use crate::ble_characteristic::{BleCharacteristicInfo, CharacteristicProperties};
use crate::ble_service::BleServiceInfo;
use crate::types::BleError;
use crate::{ble_debug, ble_info, ble_error};

/// BleDevice represents a single BLE peripheral device
///
/// This class wraps a btleplug Peripheral and provides Godot-friendly
/// methods for connecting, disconnecting, and managing the device state.
#[derive(GodotClass)]
#[class(base=RefCounted)]
pub struct BleDevice {
    base: Base<RefCounted>,

    /// The underlying btleplug peripheral
    peripheral: Arc<Peripheral>,

    /// Tokio runtime for executing async operations
    runtime: Arc<Runtime>,

    /// Device Bluetooth address
    address: GString,

    /// Device name (if available)
    name: GString,

    /// Connection state
    is_connected: Arc<Mutex<bool>>,

    /// Discovered services
    services: Arc<Mutex<Vec<BleServiceInfo>>>,

    /// Set of subscribed characteristic UUIDs (stored as lowercase strings)
    subscribed_characteristics: Arc<Mutex<HashSet<String>>>,
}

#[godot_api]
impl BleDevice {
    /// Signal emitted when device successfully connects
    #[signal]
    fn connected();

    /// Signal emitted when device disconnects
    #[signal]
    fn disconnected();

    /// Signal emitted when connection fails
    ///
    /// # Parameters
    /// * error: String - Error message describing the failure
    #[signal]
    fn connection_failed(error: GString);

    /// Signal emitted when services are discovered
    ///
    /// # Parameters
    /// * services: Array - Array of service dictionaries
    #[signal]
    fn services_discovered(services: Array<Dictionary>);

    /// Signal emitted when a characteristic is read
    ///
    /// # Parameters
    /// * char_uuid: String - UUID of the characteristic
    /// * data: PackedByteArray - Data read from the characteristic
    #[signal]
    fn characteristic_read(char_uuid: GString, data: PackedByteArray);

    /// Signal emitted when a characteristic is written
    ///
    /// # Parameters
    /// * char_uuid: String - UUID of the characteristic
    #[signal]
    fn characteristic_written(char_uuid: GString);

    /// Signal emitted when a characteristic notification is received
    ///
    /// # Parameters
    /// * char_uuid: String - UUID of the characteristic
    /// * data: PackedByteArray - Notification data
    #[signal]
    fn characteristic_notified(char_uuid: GString, data: PackedByteArray);

    /// Signal emitted when an operation fails
    ///
    /// # Parameters
    /// * operation: String - Name of the failed operation
    /// * error: String - Error message
    #[signal]
    fn operation_failed(operation: GString, error: GString);

    /// Asynchronously connect to the device
    ///
    /// This method initiates a connection to the BLE device. The result
    /// is communicated through the `connected` or `connection_failed` signals.
    #[func]
    fn connect_async(&mut self) {
        let address = self.address.to_string();
        ble_debug!("Initiating connection to device: {}", address);

        let peripheral = self.peripheral.clone();
        let runtime = self.runtime.clone();
        let is_connected = self.is_connected.clone();

        // Get instance ID for callback
        let instance_id = self.base().instance_id();

        // Execute connection asynchronously
        runtime.spawn(async move {
            ble_debug!("About to call peripheral.connect() for {}", address);
            
            let result = match peripheral.connect().await {
                Ok(_) => {
                    ble_debug!("peripheral.connect() returned Ok for {}", address);
                    *is_connected.lock().unwrap() = true;
                    ble_info!("Successfully connected to device: {}", address);
                    Ok(())
                }
                Err(e) => {
                    ble_debug!("peripheral.connect() returned Err for {}: {}", address, e);
                    Err(BleError::ConnectionFailed(e.to_string()))
                }
            };

            // Handle result on main thread
            match result {
                Ok(_) => {
                    ble_debug!("Attempting to emit connected signal for {}", address);
                    if let Ok(mut obj) = Gd::<BleDevice>::try_from_instance_id(instance_id) {
                        ble_debug!("Found device instance, calling _on_connect_success");
                        obj.call_deferred("_on_connect_success", &[]);
                    } else {
                        ble_error!("Failed to get device instance for callback");
                    }
                }
                Err(error) => {
                    ble_debug!("Connection failed for device: {}", address);
                    error.log_error();
                    
                    if let Ok(mut obj) = Gd::<BleDevice>::try_from_instance_id(instance_id) {
                        ble_debug!("Found device instance, calling _on_connect_failed");
                        obj.call_deferred(
                            "_on_connect_failed",
                            &[error.to_gstring().to_variant()],
                        );
                    } else {
                        ble_error!("Failed to get device instance for error callback");
                    }
                }
            }
        });
    }

    /// Disconnect from the device
    ///
    /// This method disconnects from the BLE device if currently connected.
    /// The `disconnected` signal is emitted upon successful disconnection.
    /// All active subscriptions are automatically cleared.
    #[func]
    fn disconnect(&mut self) {
        let address = self.address.to_string();
        ble_debug!("Initiating disconnect for device: {}", address);

        let peripheral = self.peripheral.clone();
        let runtime = self.runtime.clone();
        let is_connected = self.is_connected.clone();
        let subscribed_chars = self.subscribed_characteristics.clone();

        runtime.spawn(async move {
            match peripheral.disconnect().await {
                Ok(_) => {
                    *is_connected.lock().unwrap() = false;
                    // Clear all subscriptions
                    let sub_count = subscribed_chars.lock().unwrap().len();
                    subscribed_chars.lock().unwrap().clear();
                    ble_info!("Device {} disconnected successfully", address);
                    if sub_count > 0 {
                        ble_debug!("Cleared {} subscriptions", sub_count);
                    }
                }
                Err(e) => {
                    let error = BleError::OperationFailed(format!("Disconnect failed: {}", e));
                    error.log_warning(); // Use warning since we'll mark as disconnected anyway
                    // Still mark as disconnected since we can't maintain the connection
                    *is_connected.lock().unwrap() = false;
                    // Clear all subscriptions
                    subscribed_chars.lock().unwrap().clear();
                }
            }
        });

        // Emit signal immediately on the main thread
        self.base_mut().emit_signal("disconnected", &[]);
    }

    /// Check if the device is currently connected
    ///
    /// # Returns
    /// true if the device is connected, false otherwise
    #[func]
    pub fn is_connected(&self) -> bool {
        *self.is_connected.lock().unwrap()
    }

    /// Get the device's Bluetooth address
    ///
    /// # Returns
    /// The device address as a string
    #[func]
    fn get_address(&self) -> GString {
        self.address.clone()
    }

    /// Get the device's name
    ///
    /// # Returns
    /// The device name as a string (may be empty if name is not available)
    #[func]
    fn get_name(&self) -> GString {
        self.name.clone()
    }

    /// Discover services on the connected device
    ///
    /// This method discovers all GATT services and their characteristics
    /// on the connected device. The result is communicated through the
    /// `services_discovered` or `operation_failed` signals.
    ///
    /// The device must be connected before calling this method.
    #[func]
    fn discover_services(&mut self) {
        let address = self.address.to_string();
        ble_debug!("Starting service discovery for device: {}", address);

        // Check if connected
        if !*self.is_connected.lock().unwrap() {
            let error = BleError::NotConnected;
            error.log_error();
            self.base_mut().emit_signal(
                "operation_failed",
                &[
                    GString::from("discover_services").to_variant(),
                    error.to_gstring().to_variant(),
                ],
            );
            return;
        }

        let peripheral = self.peripheral.clone();
        let runtime = self.runtime.clone();
        let services_cache = self.services.clone();
        let instance_id = self.base().instance_id();

        // Execute service discovery asynchronously
        runtime.spawn(async move {
            ble_debug!("About to call peripheral.discover_services() for {}", address);
            
            let result = match peripheral.discover_services().await {
                Ok(_) => {
                    Ok(())
                }
                Err(e) => {
                    Err(BleError::ServiceDiscoveryFailed(e.to_string()))
                }
            };

            // Handle result on main thread
            match result {
                Ok(_) => {
                    ble_debug!("Service discovery successful for device: {}", address);
                    
                    // Get all services
                    let services = peripheral.services();
                    let mut service_infos = Vec::new();

                    for service in services {
                        let char_count = service.characteristics.len();
                        ble_debug!(
                            "Found service {} with {} characteristics",
                            service.uuid,
                            char_count
                        );

                        let char_infos: Vec<BleCharacteristicInfo> = service
                            .characteristics
                            .iter()
                            .map(|c| Self::convert_characteristic(c))
                            .collect();
                        let service_info =
                            BleServiceInfo::new(service.uuid.to_string(), char_infos);
                        service_infos.push(service_info);
                    }

                    ble_info!(
                        "Service discovery complete for device {}: found {} services",
                        address,
                        service_infos.len()
                    );

                    // Cache the services
                    *services_cache.lock().unwrap() = service_infos.clone();

                    // Convert to array of dictionaries
                    let services_array: Array<Dictionary> =
                        service_infos.iter().map(|s| s.to_dictionary()).collect();

                    // Emit signal on main thread
                    if let Ok(mut obj) = Gd::<BleDevice>::try_from_instance_id(instance_id) {
                        obj.call_deferred(
                            "_on_services_discovered",
                            &[services_array.to_variant()],
                        );
                    } else {
                        ble_error!("Failed to get device instance for services_discovered callback");
                    }
                }
                Err(error) => {
                    error.log_error();

                    // Call back to main thread to emit signal
                    if let Ok(mut obj) = Gd::<BleDevice>::try_from_instance_id(instance_id) {
                        obj.call_deferred(
                            "_on_operation_failed",
                            &[
                                GString::from("discover_services").to_variant(),
                                error.to_gstring().to_variant(),
                            ],
                        );
                    } else {
                        ble_error!("Failed to get device instance for error callback");
                    }
                }
            }
        });
    }

    /// Get the list of discovered services
    ///
    /// Returns an array of service dictionaries. Each dictionary contains:
    /// - uuid: String - The service UUID
    /// - characteristics: Array - Array of characteristic dictionaries
    ///
    /// # Returns
    /// Array of service dictionaries
    #[func]
    fn get_services(&self) -> Array<Dictionary> {
        let services = self.services.lock().unwrap();
        services.iter().map(|s| s.to_dictionary()).collect()
    }

    /// Read a characteristic value
    ///
    /// This method reads data from a characteristic identified by its service
    /// and characteristic UUIDs. The result is communicated through the
    /// `characteristic_read` or `operation_failed` signals.
    ///
    /// The device must be connected and services must be discovered before
    /// calling this method.
    ///
    /// # Parameters
    /// * service_uuid: String - UUID of the service containing the characteristic
    /// * char_uuid: String - UUID of the characteristic to read
    #[func]
    fn read_characteristic(&mut self, service_uuid: GString, char_uuid: GString) {
        ble_debug!(
            "Reading characteristic {} from service {}",
            char_uuid,
            service_uuid
        );

        // Check if connected
        if !*self.is_connected.lock().unwrap() {
            let error = BleError::NotConnected;
            error.log_error();
            self.base_mut().emit_signal(
                "operation_failed",
                &[
                    GString::from("read_characteristic").to_variant(),
                    error.to_gstring().to_variant(),
                ],
            );
            return;
        }

        let peripheral = self.peripheral.clone();
        let runtime = self.runtime.clone();
        let instance_id = self.base().instance_id();
        let service_uuid_str = service_uuid.to_string();
        let char_uuid_str = char_uuid.to_string();

        // Find the characteristic first (synchronous)
        let characteristics = peripheral.characteristics();
        let characteristic = characteristics.iter().find(|c| {
            c.uuid.to_string().eq_ignore_ascii_case(&char_uuid_str)
                && c.service_uuid
                    .to_string()
                    .eq_ignore_ascii_case(&service_uuid_str)
        });

        match characteristic {
            Some(char) => {
                ble_debug!("Found characteristic {}, reading value", char_uuid_str);
                let char_clone = char.clone();
                
                // Execute read asynchronously
                runtime.spawn(async move {
                    let result = peripheral.read(&char_clone).await;

                    match result {
                        Ok(data) => {
                            ble_info!(
                                "Successfully read {} bytes from characteristic {}",
                                data.len(),
                                char_uuid_str
                            );
                            ble_debug!("Read data: {:?}", data);

                            // Convert Vec<u8> to PackedByteArray
                            let packed_data = PackedByteArray::from(&data[..]);

                            // Call back to main thread to emit signal
                            if let Ok(mut obj) = Gd::<BleDevice>::try_from_instance_id(instance_id)
                            {
                                obj.call_deferred(
                                    "_on_characteristic_read",
                                    &[
                                        GString::from(&char_uuid_str).to_variant(),
                                        packed_data.to_variant(),
                                    ],
                                );
                            } else {
                                ble_error!("Failed to get device instance for read callback");
                            }
                        }
                        Err(e) => {
                            let error = BleError::ReadFailed(e.to_string());
                            error.log_error();

                            // Call back to main thread to emit signal
                            if let Ok(mut obj) = Gd::<BleDevice>::try_from_instance_id(instance_id)
                            {
                                obj.call_deferred(
                                    "_on_operation_failed",
                                    &[
                                        GString::from("read_characteristic").to_variant(),
                                        error.to_gstring().to_variant(),
                                    ],
                                );
                            } else {
                                ble_error!("Failed to get device instance for error callback");
                            }
                        }
                    }
                });
            }
            None => {
                let error = BleError::CharacteristicNotFound(format!(
                    "{} in service {}",
                    char_uuid_str, service_uuid_str
                ));
                error.log_error();

                // Call back to main thread to emit signal
                if let Ok(mut obj) = Gd::<BleDevice>::try_from_instance_id(instance_id) {
                    obj.call_deferred(
                        "_on_operation_failed",
                        &[
                            GString::from("read_characteristic").to_variant(),
                            error.to_gstring().to_variant(),
                        ],
                    );
                } else {
                    ble_error!("Failed to get device instance for error callback");
                }
            }
        }
    }

    /// Write data to a characteristic
    ///
    /// This method writes data to a characteristic identified by its service
    /// and characteristic UUIDs. The result is communicated through the
    /// `characteristic_written` or `operation_failed` signals.
    ///
    /// The device must be connected and services must be discovered before
    /// calling this method.
    ///
    /// # Parameters
    /// * service_uuid: String - UUID of the service containing the characteristic
    /// * char_uuid: String - UUID of the characteristic to write
    /// * data: PackedByteArray - Data to write to the characteristic
    /// * with_response: bool - If true, wait for write response; if false, write without response
    #[func]
    fn write_characteristic(
        &mut self,
        service_uuid: GString,
        char_uuid: GString,
        data: PackedByteArray,
        with_response: bool,
    ) {
        let write_mode = if with_response { "with response" } else { "without response" };
        ble_debug!(
            "Writing {} bytes to characteristic {} ({}) from service {}",
            data.len(),
            char_uuid,
            write_mode,
            service_uuid
        );

        // Check if connected
        if !*self.is_connected.lock().unwrap() {
            let error = BleError::NotConnected;
            error.log_error();
            self.base_mut().emit_signal(
                "operation_failed",
                &[
                    GString::from("write_characteristic").to_variant(),
                    error.to_gstring().to_variant(),
                ],
            );
            return;
        }

        let peripheral = self.peripheral.clone();
        let runtime = self.runtime.clone();
        let instance_id = self.base().instance_id();
        let service_uuid_str = service_uuid.to_string();
        let char_uuid_str = char_uuid.to_string();

        // Convert PackedByteArray to Vec<u8>
        let data_vec: Vec<u8> = data.to_vec();
        let data_len = data_vec.len();

        // Find the characteristic first (synchronous)
        let characteristics = peripheral.characteristics();
        let characteristic = characteristics.iter().find(|c| {
            c.uuid.to_string().eq_ignore_ascii_case(&char_uuid_str)
                && c.service_uuid
                    .to_string()
                    .eq_ignore_ascii_case(&service_uuid_str)
        });

        match characteristic {
            Some(char) => {
                ble_debug!("Found characteristic {}, writing data", char_uuid_str);
                ble_debug!("Write data: {:?}", data_vec);
                let char_clone = char.clone();

                // Execute write asynchronously
                runtime.spawn(async move {
                    // Choose write method based on with_response parameter
                    let result = if with_response {
                        peripheral
                            .write(&char_clone, &data_vec, WriteType::WithResponse)
                            .await
                    } else {
                        peripheral
                            .write(&char_clone, &data_vec, WriteType::WithoutResponse)
                            .await
                    };

                    match result {
                        Ok(_) => {
                            ble_info!(
                                "Successfully wrote {} bytes to characteristic {}",
                                data_len,
                                char_uuid_str
                            );

                            // Call back to main thread to emit signal
                            if let Ok(mut obj) = Gd::<BleDevice>::try_from_instance_id(instance_id)
                            {
                                obj.call_deferred(
                                    "_on_characteristic_written",
                                    &[GString::from(&char_uuid_str).to_variant()],
                                );
                            } else {
                                ble_error!("Failed to get device instance for write callback");
                            }
                        }
                        Err(e) => {
                            let error = BleError::WriteFailed(e.to_string());
                            error.log_error();

                            // Call back to main thread to emit signal
                            if let Ok(mut obj) = Gd::<BleDevice>::try_from_instance_id(instance_id)
                            {
                                obj.call_deferred(
                                    "_on_operation_failed",
                                    &[
                                        GString::from("write_characteristic").to_variant(),
                                        error.to_gstring().to_variant(),
                                    ],
                                );
                            } else {
                                ble_error!("Failed to get device instance for error callback");
                            }
                        }
                    }
                });
            }
            None => {
                let error = BleError::CharacteristicNotFound(format!(
                    "{} in service {}",
                    char_uuid_str, service_uuid_str
                ));
                error.log_error();

                // Call back to main thread to emit signal
                if let Ok(mut obj) = Gd::<BleDevice>::try_from_instance_id(instance_id) {
                    obj.call_deferred(
                        "_on_operation_failed",
                        &[
                            GString::from("write_characteristic").to_variant(),
                            error.to_gstring().to_variant(),
                        ],
                    );
                } else {
                    ble_error!("Failed to get device instance for error callback");
                }
            }
        }
    }

    // Internal callback methods called from async tasks

    #[func]
    fn _on_connect_success(&mut self) {
        self.base_mut().emit_signal("connected", &[]);
    }

    #[func]
    fn _on_connect_failed(&mut self, error: GString) {
        self.base_mut()
            .emit_signal("connection_failed", &[error.to_variant()]);
    }

    #[func]
    fn _on_services_discovered(&mut self, services: Array<Dictionary>) {
        self.base_mut()
            .emit_signal("services_discovered", &[services.to_variant()]);
    }

    #[func]
    fn _on_operation_failed(&mut self, operation: GString, error: GString) {
        self.base_mut().emit_signal(
            "operation_failed",
            &[operation.to_variant(), error.to_variant()],
        );
    }

    #[func]
    fn _on_characteristic_read(&mut self, char_uuid: GString, data: PackedByteArray) {
        self.base_mut().emit_signal(
            "characteristic_read",
            &[char_uuid.to_variant(), data.to_variant()],
        );
    }

    #[func]
    fn _on_characteristic_written(&mut self, char_uuid: GString) {
        self.base_mut()
            .emit_signal("characteristic_written", &[char_uuid.to_variant()]);
    }

    #[func]
    fn _on_characteristic_notified(&mut self, char_uuid: GString, data: PackedByteArray) {
        self.base_mut().emit_signal(
            "characteristic_notified",
            &[char_uuid.to_variant(), data.to_variant()],
        );
    }

    /// Subscribe to characteristic notifications
    ///
    /// This method subscribes to notifications from a characteristic identified
    /// by its service and characteristic UUIDs. When notifications are received,
    /// they are delivered through the `characteristic_notified` signal.
    ///
    /// The device must be connected and services must be discovered before
    /// calling this method. The characteristic must support notifications or
    /// indications.
    ///
    /// Multiple characteristics can be subscribed simultaneously.
    ///
    /// # Parameters
    /// * service_uuid: String - UUID of the service containing the characteristic
    /// * char_uuid: String - UUID of the characteristic to subscribe to
    #[func]
    fn subscribe_characteristic(&mut self, service_uuid: GString, char_uuid: GString) {
        ble_debug!(
            "Subscribing to characteristic {} from service {}",
            char_uuid,
            service_uuid
        );

        // Check if connected
        if !*self.is_connected.lock().unwrap() {
            let error = BleError::NotConnected;
            error.log_error();
            self.base_mut().emit_signal(
                "operation_failed",
                &[
                    GString::from("subscribe_characteristic").to_variant(),
                    error.to_gstring().to_variant(),
                ],
            );
            return;
        }

        let peripheral = self.peripheral.clone();
        let runtime = self.runtime.clone();
        let instance_id = self.base().instance_id();
        let service_uuid_str = service_uuid.to_string();
        let char_uuid_str = char_uuid.to_string();
        let subscribed_chars = self.subscribed_characteristics.clone();

        // Find the characteristic first (synchronous)
        let characteristics = peripheral.characteristics();
        let characteristic = characteristics.iter().find(|c| {
            c.uuid.to_string().eq_ignore_ascii_case(&char_uuid_str)
                && c.service_uuid
                    .to_string()
                    .eq_ignore_ascii_case(&service_uuid_str)
        });

        match characteristic {
            Some(char) => {
                ble_debug!("Found characteristic {}, subscribing", char_uuid_str);
                let char_clone = char.clone();
                
                // Execute subscribe asynchronously
                runtime.spawn(async move {
                    let result = peripheral.subscribe(&char_clone).await;

                    match result {
                        Ok(_) => {
                            // Add to subscribed set
                            subscribed_chars
                                .lock()
                                .unwrap()
                                .insert(char_uuid_str.to_lowercase());

                            ble_info!(
                                "Successfully subscribed to characteristic {}",
                                char_uuid_str
                            );

                            // Set up notification handler
                            let peripheral_clone = peripheral.clone();
                            let char_uuid_for_handler = char_uuid_str.clone();

                            tokio::spawn(async move {
                                ble_debug!("Starting notification handler for {}", char_uuid_for_handler);
                                let mut notification_stream = peripheral_clone.notifications().await;

                                if let Ok(stream) = notification_stream.as_mut() {
                                    use futures::StreamExt;
                                    while let Some(notification) = stream.next().await {
                                        // Check if this notification is for our characteristic
                                        if notification
                                            .uuid
                                            .to_string()
                                            .eq_ignore_ascii_case(&char_uuid_for_handler)
                                        {
                                            ble_debug!(
                                                "Received notification from {}: {} bytes",
                                                char_uuid_for_handler,
                                                notification.value.len()
                                            );

                                            let packed_data =
                                                PackedByteArray::from(&notification.value[..]);

                                            // Call back to main thread to emit signal
                                            if let Ok(mut obj) =
                                                Gd::<BleDevice>::try_from_instance_id(instance_id)
                                            {
                                                obj.call_deferred(
                                                    "_on_characteristic_notified",
                                                    &[
                                                        GString::from(&char_uuid_for_handler)
                                                            .to_variant(),
                                                        packed_data.to_variant(),
                                                    ],
                                                );
                                            } else {
                                                ble_error!("Failed to get device instance for notification callback");
                                            }
                                        }
                                    }
                                    ble_debug!("Notification stream ended for {}", char_uuid_for_handler);
                                } else {
                                    ble_error!("Failed to get notification stream for {}", char_uuid_for_handler);
                                }
                            });
                        }
                        Err(e) => {
                            let error = BleError::SubscribeFailed(e.to_string());
                            error.log_error();

                            // Call back to main thread to emit signal
                            if let Ok(mut obj) = Gd::<BleDevice>::try_from_instance_id(instance_id)
                            {
                                obj.call_deferred(
                                    "_on_operation_failed",
                                    &[
                                        GString::from("subscribe_characteristic").to_variant(),
                                        error.to_gstring().to_variant(),
                                    ],
                                );
                            } else {
                                ble_error!("Failed to get device instance for error callback");
                            }
                        }
                    }
                });
            }
            None => {
                let error = BleError::CharacteristicNotFound(format!(
                    "{} in service {}",
                    char_uuid_str, service_uuid_str
                ));
                error.log_error();

                // Call back to main thread to emit signal
                if let Ok(mut obj) = Gd::<BleDevice>::try_from_instance_id(instance_id) {
                    obj.call_deferred(
                        "_on_operation_failed",
                        &[
                            GString::from("subscribe_characteristic").to_variant(),
                            error.to_gstring().to_variant(),
                        ],
                    );
                } else {
                    ble_error!("Failed to get device instance for error callback");
                }
            }
        }
    }

    /// Unsubscribe from characteristic notifications
    ///
    /// This method unsubscribes from notifications for a characteristic
    /// identified by its service and characteristic UUIDs. After unsubscribing,
    /// no more notifications will be received for this characteristic.
    ///
    /// The device must be connected before calling this method.
    ///
    /// # Parameters
    /// * service_uuid: String - UUID of the service containing the characteristic
    /// * char_uuid: String - UUID of the characteristic to unsubscribe from
    #[func]
    fn unsubscribe_characteristic(&mut self, service_uuid: GString, char_uuid: GString) {
        ble_debug!(
            "Unsubscribing from characteristic {} from service {}",
            char_uuid,
            service_uuid
        );

        // Check if connected
        if !*self.is_connected.lock().unwrap() {
            let error = BleError::NotConnected;
            error.log_error();
            self.base_mut().emit_signal(
                "operation_failed",
                &[
                    GString::from("unsubscribe_characteristic").to_variant(),
                    error.to_gstring().to_variant(),
                ],
            );
            return;
        }

        let peripheral = self.peripheral.clone();
        let runtime = self.runtime.clone();
        let instance_id = self.base().instance_id();
        let service_uuid_str = service_uuid.to_string();
        let char_uuid_str = char_uuid.to_string();
        let subscribed_chars = self.subscribed_characteristics.clone();

        // Find the characteristic first (synchronous)
        let characteristics = peripheral.characteristics();
        let characteristic = characteristics.iter().find(|c| {
            c.uuid.to_string().eq_ignore_ascii_case(&char_uuid_str)
                && c.service_uuid
                    .to_string()
                    .eq_ignore_ascii_case(&service_uuid_str)
        });

        match characteristic {
            Some(char) => {
                ble_debug!("Found characteristic {}, unsubscribing", char_uuid_str);
                let char_clone = char.clone();
                
                // Execute unsubscribe asynchronously
                runtime.spawn(async move {
                    let result = peripheral.unsubscribe(&char_clone).await;

                    match result {
                        Ok(_) => {
                            // Remove from subscribed set
                            subscribed_chars
                                .lock()
                                .unwrap()
                                .remove(&char_uuid_str.to_lowercase());

                            ble_info!(
                                "Successfully unsubscribed from characteristic {}",
                                char_uuid_str
                            );
                        }
                        Err(e) => {
                            let error = BleError::UnsubscribeFailed(e.to_string());
                            error.log_error();

                            // Call back to main thread to emit signal
                            if let Ok(mut obj) = Gd::<BleDevice>::try_from_instance_id(instance_id)
                            {
                                obj.call_deferred(
                                    "_on_operation_failed",
                                    &[
                                        GString::from("unsubscribe_characteristic").to_variant(),
                                        error.to_gstring().to_variant(),
                                    ],
                                );
                            } else {
                                ble_error!("Failed to get device instance for error callback");
                            }
                        }
                    }
                });
            }
            None => {
                let error = BleError::CharacteristicNotFound(format!(
                    "{} in service {}",
                    char_uuid_str, service_uuid_str
                ));
                error.log_error();

                // Call back to main thread to emit signal
                if let Ok(mut obj) = Gd::<BleDevice>::try_from_instance_id(instance_id) {
                    obj.call_deferred(
                        "_on_operation_failed",
                        &[
                            GString::from("unsubscribe_characteristic").to_variant(),
                            error.to_gstring().to_variant(),
                        ],
                    );
                } else {
                    ble_error!("Failed to get device instance for error callback");
                }
            }
        }
    }
}

#[godot_api]
impl IRefCounted for BleDevice {
    /// Initialize the BleDevice
    ///
    /// Note: This is called by Godot when the object is created.
    /// Use the `new` method to create instances from Rust code.
    fn init(_base: Base<RefCounted>) -> Self {
        // This should not be called directly - use new() instead
        // We panic here because BleDevice must be created through the new() method
        panic!("BleDevice::init called directly - use BleDevice::new() instead");
    }
}

/// Implement Drop trait to handle cleanup when the device is destroyed
impl Drop for BleDevice {
    fn drop(&mut self) {
        ble_info!("BleDevice: Cleaning up resources for device {}", self.address);
        self.cleanup();
    }
}

impl BleDevice {
    /// Create a new BleDevice instance
    ///
    /// This is the proper way to create a BleDevice from Rust code.
    ///
    /// # Parameters
    /// * peripheral: The btleplug Peripheral to wrap
    /// * runtime: The Tokio runtime to use for async operations
    ///
    /// # Returns
    /// A new Gd<BleDevice> instance
    pub fn new(peripheral: Peripheral, runtime: Arc<Runtime>) -> Gd<Self> {
        // On macOS, use UUID as address since MAC address is not exposed
        let address = peripheral.id().to_string();
        
        // Get device properties
        let properties = runtime.block_on(async { peripheral.properties().await });

        let name = match properties {
            Ok(Some(props)) => props.local_name.unwrap_or_default(),
            Ok(None) => String::new(),
            Err(e) => {
                godot_warn!("Failed to get device properties: {}", e);
                String::new()
            }
        };

        Gd::from_init_fn(|base| Self {
            base,
            peripheral: Arc::new(peripheral),
            runtime,
            address: GString::from(&address),
            name: GString::from(&name),
            is_connected: Arc::new(Mutex::new(false)),
            services: Arc::new(Mutex::new(Vec::new())),
            subscribed_characteristics: Arc::new(Mutex::new(HashSet::new())),
        })
    }

    /// Convert a btleplug characteristic to BleCharacteristicInfo
    fn convert_characteristic(characteristic: &Characteristic) -> BleCharacteristicInfo {
        let props = CharacteristicProperties {
            read: characteristic.properties.contains(CharPropFlags::READ),
            write: characteristic.properties.contains(CharPropFlags::WRITE),
            write_without_response: characteristic
                .properties
                .contains(CharPropFlags::WRITE_WITHOUT_RESPONSE),
            notify: characteristic.properties.contains(CharPropFlags::NOTIFY),
            indicate: characteristic.properties.contains(CharPropFlags::INDICATE),
        };
        BleCharacteristicInfo::new(characteristic.uuid.to_string(), props)
    }

    /// Clean up resources when the device is destroyed
    ///
    /// This method is called automatically when the device object is about to be destroyed.
    /// It ensures proper cleanup of all subscriptions and disconnects the device.
    fn cleanup(&mut self) {
        let address = self.address.to_string();
        ble_debug!("Starting cleanup for device: {}", address);

        // Check if device is connected
        let is_connected = *self.is_connected.lock().unwrap();
        
        if is_connected {
            ble_debug!("Device {} is connected, initiating disconnect", address);
            
            // Get subscription count before clearing
            let sub_count = self.subscribed_characteristics.lock().unwrap().len();
            if sub_count > 0 {
                ble_debug!("Clearing {} active subscriptions", sub_count);
            }

            // Unsubscribe from all characteristics
            let subscribed_uuids: Vec<String> = self
                .subscribed_characteristics
                .lock()
                .unwrap()
                .iter()
                .cloned()
                .collect();

            let peripheral = self.peripheral.clone();
            let runtime = self.runtime.clone();
            let subscribed_chars = self.subscribed_characteristics.clone();
            let address_clone = address.clone();

            // Perform cleanup in async context
            runtime.spawn(async move {
                // Unsubscribe from all characteristics
                if !subscribed_uuids.is_empty() {
                    ble_debug!("Unsubscribing from {} characteristics", subscribed_uuids.len());
                    let characteristics = peripheral.characteristics();
                    
                    for uuid in &subscribed_uuids {
                        if let Some(char) = characteristics.iter().find(|c| {
                            c.uuid.to_string().eq_ignore_ascii_case(uuid)
                        }) {
                            match peripheral.unsubscribe(char).await {
                                Ok(_) => {
                                    ble_debug!("Unsubscribed from characteristic {}", uuid);
                                }
                                Err(e) => {
                                    ble_debug!("Failed to unsubscribe from {}: {}", uuid, e);
                                }
                            }
                        }
                    }
                }

                // Clear the subscribed set
                subscribed_chars.lock().unwrap().clear();

                // Disconnect the device
                match peripheral.disconnect().await {
                    Ok(_) => {
                        ble_info!("Device {} disconnected during cleanup", address_clone);
                    }
                    Err(e) => {
                        ble_debug!("Disconnect during cleanup failed for {}: {}", address_clone, e);
                    }
                }
            });

            // Mark as disconnected immediately
            *self.is_connected.lock().unwrap() = false;
        } else {
            ble_debug!("Device {} already disconnected, skipping cleanup", address);
        }

        // Clear services cache
        self.services.lock().unwrap().clear();
        
        ble_info!("Cleanup complete for device: {}", address);
    }
}
