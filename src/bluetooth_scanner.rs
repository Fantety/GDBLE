use btleplug::api::{Central, CentralEvent, ScanFilter};
use btleplug::platform::Adapter;
use futures::StreamExt;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::runtime::Runtime;
use tokio::sync::mpsc;
use tokio::time::timeout;

use crate::types::{BleError, DeviceInfo};
use crate::{ble_debug, ble_info, ble_error, ble_warn};

/// BluetoothScanner handles BLE device scanning operations
/// 
/// This struct manages the scanning state and discovered devices.
/// It is not a GodotClass and is used internally by BluetoothManager.
pub struct BluetoothScanner {
    /// The Bluetooth adapter used for scanning
    adapter: Arc<Adapter>,
    
    /// Tokio runtime for async operations
    runtime: Arc<Runtime>,
    
    /// Current scanning state
    is_scanning: Arc<Mutex<bool>>,
    
    /// Map of discovered devices by address
    discovered_devices: Arc<Mutex<HashMap<String, DeviceInfo>>>,
}

impl BluetoothScanner {
    /// Creates a new BluetoothScanner instance
    /// 
    /// # Parameters
    /// * `adapter` - The Bluetooth adapter to use for scanning
    /// * `runtime` - The Tokio runtime for executing async operations
    /// 
    /// # Returns
    /// A new BluetoothScanner instance
    pub fn new(adapter: Arc<Adapter>, runtime: Arc<Runtime>) -> Self {
        Self {
            adapter,
            runtime,
            is_scanning: Arc::new(Mutex::new(false)),
            discovered_devices: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Starts scanning for BLE devices
    /// 
    /// This method initiates a BLE scan that will run for the specified duration.
    /// Discovered devices are sent through the provided channel as they are found.
    /// 
    /// # Parameters
    /// * `scan_duration` - How long to scan for devices
    /// * `device_tx` - Channel sender for discovered devices
    /// 
    /// # Returns
    /// Ok(()) if scanning started successfully, Err otherwise
    pub async fn start_scan(
        &self,
        scan_duration: Duration,
        device_tx: mpsc::UnboundedSender<DeviceInfo>,
    ) -> Result<(), BleError> {
        ble_debug!("Starting BLE scan for {:?}", scan_duration);

        // Check if already scanning
        {
            let mut scanning = self.is_scanning.lock()
                .map_err(|e| {
                    let error = BleError::InternalError(format!("Lock error: {}", e));
                    error.log_error();
                    error
                })?;
            
            if *scanning {
                let error = BleError::ScanFailed("Already scanning".to_string());
                error.log_warning();
                return Err(error);
            }
            
            *scanning = true;
        }

        // Clear previous scan results
        {
            let mut devices = self.discovered_devices.lock()
                .map_err(|e| {
                    let error = BleError::InternalError(format!("Lock error: {}", e));
                    error.log_error();
                    error
                })?;
            let prev_count = devices.len();
            devices.clear();
            if prev_count > 0 {
                ble_debug!("Cleared {} previous scan results", prev_count);
            }
        }

        // Start scanning
        ble_debug!("Initiating adapter scan");
        ble_debug!("Scan filter: {:?}", ScanFilter::default());
        
        let scan_result = self.adapter.start_scan(ScanFilter::default()).await;
        match &scan_result {
            Ok(_) => ble_info!("Adapter start_scan() returned Ok"),
            Err(e) => ble_error!("Adapter start_scan() returned Err: {}", e),
        }
        
        scan_result.map_err(|e| {
            let error = BleError::ScanFailed(e.to_string());
            error.log_error();
            error
        })?;

        ble_info!("BLE scan started successfully");

        // Wait for scan duration with additional debugging
        ble_debug!("Waiting for scan duration: {:?}", scan_duration);
        let result = timeout(scan_duration, self.collect_devices(device_tx)).await;
        ble_debug!("Scan timeout completed, checking result...");

        // Stop scanning
        ble_debug!("Stopping adapter scan");
        let stop_result = self.adapter.stop_scan().await;

        // Update scanning state
        {
            let mut scanning = self.is_scanning.lock()
                .map_err(|e| {
                    let error = BleError::InternalError(format!("Lock error: {}", e));
                    error.log_error();
                    error
                })?;
            *scanning = false;
        }

        // Check for errors
        stop_result.map_err(|e| {
            let error = BleError::ScanFailed(format!("Failed to stop scan: {}", e));
            error.log_error();
            error
        })?;
        
        match result {
            Ok(Ok(())) => {
                ble_debug!("Scan collection completed successfully");
                Ok(())
            }
            Ok(Err(e)) => {
                e.log_error();
                Err(e)
            }
            Err(_) => {
                ble_debug!("Scan timeout reached (expected)");
                Ok(())
            }
        }
    }

    /// Stops an ongoing scan
    /// 
    /// This method stops the current BLE scan if one is in progress.
    pub fn stop_scan(&self) {
        // Update scanning state
        if let Ok(mut scanning) = self.is_scanning.lock() {
            if !*scanning {
                return; // Not scanning
            }
            *scanning = false;
        }

        // Stop scan asynchronously
        let adapter = self.adapter.clone();
        let runtime = self.runtime.clone();
        
        runtime.spawn(async move {
            let _ = adapter.stop_scan().await;
        });
    }

    /// Collects devices during scanning
    /// 
    /// This internal method listens for device discovery events and sends them
    /// through the channel immediately, while also updating the discovered_devices map.
    async fn collect_devices(&self, device_tx: mpsc::UnboundedSender<DeviceInfo>) -> Result<(), BleError> {
        use btleplug::api::Peripheral as _;
        
        ble_debug!("Starting device collection");
        
        // Get events stream
        let mut events = self.adapter.events().await
            .map_err(|e| {
                let error = BleError::ScanFailed(format!("Failed to get events: {}", e));
                error.log_error();
                error
            })?;

        ble_debug!("Events stream created successfully");

        // Process events with a counter to avoid infinite loops
        let mut event_count = 0;
        let mut device_discovery_count = 0;
        let max_events = 1000; // Safety limit to prevent infinite loops
        
        while let Some(event) = events.next().await {
            event_count += 1;
            if event_count > max_events {
                ble_warn!("Event limit reached, stopping collection to prevent infinite loop");
                break;
            }
            
            // Only log every 10th event to reduce spam
            if event_count % 10 == 0 {
                ble_debug!("Processing event #{}: {:?}", event_count, event);
            }
            
            match event {
                CentralEvent::DeviceDiscovered(id) => {
                    device_discovery_count += 1;
                    // Get peripheral
                    if let Ok(peripheral) = self.adapter.peripheral(&id).await {
                        // Get properties
                        if let Ok(Some(properties)) = peripheral.properties().await {
                            // On macOS, use UUID as address since MAC address is not exposed
                            let address = id.to_string();
                            let name = properties.local_name.clone();
                            let rssi = properties.rssi;

                            ble_info!(
                                "Discovered device: {} ({}), RSSI: {:?}",
                                name.as_ref().unwrap_or(&"Unknown".to_string()),
                                address,
                                rssi
                            );

                            let device_info = DeviceInfo::new(address.clone(), name, rssi);

                            // Store device
                            if let Ok(mut devices) = self.discovered_devices.lock() {
                                devices.insert(address.clone(), device_info.clone());
                            } else {
                                ble_error!("Failed to acquire device map lock");
                            }

                            // Send device info immediately through channel
                            if device_tx.send(device_info).is_err() {
                                ble_warn!("Failed to send device info through channel");
                            }
                        } else {
                            ble_debug!("Failed to get properties for device {:?}", id);
                        }
                    } else {
                        ble_debug!("Failed to get peripheral for device {:?}", id);
                    }
                }
                CentralEvent::DeviceUpdated(id) => {
                    // Get peripheral
                    if let Ok(peripheral) = self.adapter.peripheral(&id).await {
                        // Get properties
                        if let Ok(Some(properties)) = peripheral.properties().await {
                            // On macOS, use UUID as address since MAC address is not exposed
                            let address = id.to_string();
                            let name = properties.local_name.clone();
                            let rssi = properties.rssi;

                            ble_debug!(
                                "Updated device: {} ({}), RSSI: {:?}",
                                name.as_ref().unwrap_or(&"Unknown".to_string()),
                                address,
                                rssi
                            );

                            let device_info = DeviceInfo::new(address.clone(), name, rssi);

                            // Update device
                            let should_send = if let Ok(mut devices) = self.discovered_devices.lock() {
                                // Only send update if device already exists (to avoid duplicates)
                                let exists = devices.contains_key(&address);
                                devices.insert(address.clone(), device_info.clone());
                                exists
                            } else {
                                ble_error!("Failed to acquire device map lock");
                                false
                            };

                            // Send device update through channel if it's an existing device
                            if should_send {
                                if device_tx.send(device_info).is_err() {
                                    ble_warn!("Failed to send device update through channel");
                                }
                            }
                        }
                    }
                }
                _ => {
                    // Only log non-discovery events every 50th time
                    if event_count % 50 == 0 {
                        ble_debug!("Other event received: {:?}", event);
                    }
                }
            }
        }

        ble_info!("Device collection completed. Discovered {} unique devices from {} events", device_discovery_count, event_count);
        Ok(())
    }

    /// Gets all discovered devices
    /// 
    /// # Returns
    /// A vector of DeviceInfo for all discovered devices
    pub fn get_devices(&self) -> Vec<DeviceInfo> {
        if let Ok(devices) = self.discovered_devices.lock() {
            devices.values().cloned().collect()
        } else {
            Vec::new()
        }
    }

    /// Checks if currently scanning
    /// 
    /// # Returns
    /// true if a scan is in progress, false otherwise
    pub fn is_scanning(&self) -> bool {
        if let Ok(scanning) = self.is_scanning.lock() {
            *scanning
        } else {
            false
        }
    }
}
