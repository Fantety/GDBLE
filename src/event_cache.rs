//! Pure cache projections, applied before invoking any Godot signal handler.

use crate::ble_service::BleServiceInfo;
use crate::core::{BleEvent, ConnectionState, EventData, Operation, OperationId, Phase};

pub struct DeviceCache {
    pub state: ConnectionState,
    pub services: Vec<BleServiceInfo>,
}

impl Default for DeviceCache {
    fn default() -> Self {
        Self {
            state: ConnectionState::Disconnected,
            services: Vec::new(),
        }
    }
}

impl DeviceCache {
    pub fn apply(&mut self, event: &BleEvent) {
        if event
            .error
            .as_ref()
            .is_some_and(|error| error.code == "SERVICES_CHANGED")
        {
            self.services.clear();
        }
        if let Some(state) = event.connection_state {
            self.state = state;
            if state == ConnectionState::Disconnected {
                self.services.clear();
            }
        }
        if let (Operation::DiscoverServices, Phase::Succeeded, EventData::Services(services)) =
            (event.operation, event.phase, &event.data)
        {
            self.services = services.clone();
        }
    }
}

pub struct ScanCache {
    pub active_id: OperationId,
    started: bool,
}

impl Default for ScanCache {
    fn default() -> Self {
        Self {
            active_id: OperationId::UNSOLICITED,
            started: false,
        }
    }
}

#[derive(Default)]
pub struct ScanSignals {
    pub started: bool,
    pub stopped: bool,
}

impl ScanCache {
    pub fn submitted(&mut self, id: OperationId) {
        if self.active_id == OperationId::UNSOLICITED {
            self.active_id = id;
            self.started = false;
        }
    }

    pub fn apply(&mut self, event: &BleEvent) -> ScanSignals {
        let mut signals = ScanSignals::default();
        if event.operation != Operation::Scan || event.operation_id == OperationId::UNSOLICITED {
            return signals;
        }
        // A new scan can be accepted before the main thread consumes the old
        // terminal event. The core's started event is authoritative in that case.
        if !event.terminal && event.phase == Phase::Started {
            signals.started = !self.started || self.active_id != event.operation_id;
            self.active_id = event.operation_id;
            self.started = true;
            return signals;
        }
        if event.operation_id != self.active_id {
            return signals;
        }
        if event.terminal {
            signals.stopped = self.started;
            self.active_id = OperationId::UNSOLICITED;
            self.started = false;
        }
        signals
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{BleError, EventContext};

    #[test]
    fn local_failures_do_not_invent_or_erase_connections() {
        for initial in [ConnectionState::Disconnected, ConnectionState::Connected] {
            let mut cache = DeviceCache {
                state: initial,
                ..DeviceCache::default()
            };
            for operation in [Operation::Connect, Operation::Disconnect] {
                cache.apply(&BleEvent::failed(
                    operation,
                    OperationId::new(1),
                    EventContext::default(),
                    BleError::QueueFull,
                ));
                assert_eq!(cache.state, initial);
            }
        }
    }

    #[test]
    fn service_cache_is_updated_and_cleared_with_the_connection_snapshot() {
        let mut cache = DeviceCache::default();
        let service = BleServiceInfo::new("service".to_string(), Vec::new());
        let mut discovered = BleEvent::succeeded(
            Operation::DiscoverServices,
            OperationId::new(1),
            EventContext::default(),
            EventData::Services(vec![service]),
        );
        discovered.connection_state = Some(ConnectionState::Connected);
        cache.apply(&discovered);
        assert_eq!(cache.state, ConnectionState::Connected);
        assert_eq!(cache.services.len(), 1);
        let mut disconnected = BleEvent::received(
            Operation::Disconnect,
            EventContext::default(),
            EventData::None,
        );
        disconnected.connection_state = Some(ConnectionState::Disconnected);
        cache.apply(&disconnected);
        assert_eq!(cache.state, ConnectionState::Disconnected);
        assert!(cache.services.is_empty());
    }

    #[test]
    fn scan_started_restores_identity_after_a_delayed_terminal_event() {
        let mut cache = ScanCache::default();
        let old = OperationId::new(1);
        let new = OperationId::new(2);
        cache.submitted(old);
        cache.submitted(new);
        cache.apply(&BleEvent::failed(
            Operation::Scan,
            old,
            EventContext::default(),
            BleError::NotInitialized,
        ));
        assert!(
            cache
                .apply(&BleEvent::started(
                    Operation::Scan,
                    new,
                    EventContext::default(),
                ))
                .started
        );
        assert_eq!(cache.active_id, new);
        assert!(
            cache
                .apply(&BleEvent::succeeded(
                    Operation::Scan,
                    new,
                    EventContext::default(),
                    EventData::None,
                ))
                .stopped
        );
        assert_eq!(cache.active_id, OperationId::UNSOLICITED);
    }

    #[test]
    fn scan_errors_and_rejected_duplicates_do_not_end_the_active_scan() {
        let mut cache = ScanCache::default();
        let id = OperationId::new(1);
        cache.submitted(id);
        assert!(
            cache
                .apply(&BleEvent::started(
                    Operation::Scan,
                    id,
                    EventContext::default()
                ))
                .started
        );
        let mut transient = BleEvent::failed(
            Operation::Scan,
            id,
            EventContext::default(),
            BleError::ScanFailed("transient".to_string()),
        );
        transient.terminal = false;
        assert!(!cache.apply(&transient).stopped);
        assert_eq!(cache.active_id, id);
        let duplicate = BleEvent::failed(
            Operation::Scan,
            OperationId::new(2),
            EventContext::default(),
            BleError::Busy("active scan".to_string()),
        );
        assert!(!cache.apply(&duplicate).stopped);
        assert_eq!(cache.active_id, id);
        transient.terminal = true;
        assert!(cache.apply(&transient).stopped);
        assert_eq!(cache.active_id, OperationId::UNSOLICITED);
        assert!(!cache.apply(&transient).stopped);
    }
}
