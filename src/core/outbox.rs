//! Admission-controlled delivery. The coordinator never waits for Godot.
use super::*;
use std::sync::Mutex;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(super) enum SignalKey {
    Adapter,
    Scan,
    Device(DeviceId),
    Notification(DeviceId),
}

struct RequestSlots {
    started: Option<mpsc::OwnedPermit<BleEvent>>,
    terminal: mpsc::OwnedPermit<BleEvent>,
}

struct SignalSlot {
    permit: Option<mpsc::OwnedPermit<BleEvent>>,
    pending: Option<BleEvent>,
    retired: bool,
}

#[derive(Default)]
struct Reservations {
    requests: HashMap<OperationId, RequestSlots>,
    signals: HashMap<SignalKey, SignalSlot>,
    scans: VecDeque<BleEvent>,
    notification_generations: HashMap<DeviceId, u64>,
    discovery_generation: u64,
}

pub(super) struct EventOutbox {
    sender: mpsc::Sender<BleEvent>,
    reservations: Mutex<Reservations>,
}

impl EventOutbox {
    pub(super) fn new(sender: mpsc::Sender<BleEvent>) -> Self {
        Self {
            sender,
            reservations: Mutex::new(Reservations::default()),
        }
    }

    fn slot(&self) -> Result<mpsc::OwnedPermit<BleEvent>, BleError> {
        self.sender
            .clone()
            .try_reserve_owned()
            .map_err(|error| match error {
                mpsc::error::TrySendError::Full(_) => BleError::QueueFull,
                mpsc::error::TrySendError::Closed(_) => {
                    BleError::Cancelled("BLE core is closed".into())
                }
            })
    }

    pub(super) fn reserve_request(&self, id: OperationId) -> Result<(), BleError> {
        let started = self.slot()?;
        let terminal = self.slot()?; // Partial reservation is dropped on failure.
        self.reservations.lock().unwrap().requests.insert(
            id,
            RequestSlots {
                started: Some(started),
                terminal,
            },
        );
        Ok(())
    }

    pub(super) fn release_request(&self, id: OperationId) {
        self.reservations.lock().unwrap().requests.remove(&id);
    }

    pub(super) fn reserve_signal(&self, key: SignalKey) -> Result<(), BleError> {
        let mut state = self.reservations.lock().unwrap();
        if let Some(slot) = state.signals.get_mut(&key) {
            if slot.pending.is_some() {
                return Err(BleError::QueueFull);
            }
            if slot.permit.is_none() {
                slot.permit = Some(self.slot()?);
            }
            slot.retired = false;
            return Ok(());
        }
        // Reusable state mailboxes are bounded too, including undelivered
        // notices belonging to sessions that have already ended.
        if state.signals.len() >= EVENT_CHANNEL_CAPACITY {
            return Err(BleError::QueueFull);
        }
        state.signals.insert(
            key,
            SignalSlot {
                permit: Some(self.slot()?),
                pending: None,
                retired: false,
            },
        );
        Ok(())
    }

    pub(super) fn retire_signal(&self, key: &SignalKey) {
        let mut state = self.reservations.lock().unwrap();
        if let Some(slot) = state.signals.get_mut(key) {
            slot.retired = true;
            slot.permit = None;
            if slot.pending.is_none() {
                state.signals.remove(key);
            }
        }
    }

    pub(super) fn request(&self, event: BleEvent) {
        let mut state = self.reservations.lock().unwrap();
        if event.terminal {
            if event.operation == Operation::Scan {
                state
                    .scans
                    .retain(|pending| pending.operation_id != event.operation_id);
            }
            if let Some(slots) = state.requests.remove(&event.operation_id) {
                slots.terminal.send(event);
            }
        } else if let Some(slots) = state.requests.get_mut(&event.operation_id) {
            if let Some(slot) = slots.started.take() {
                slot.send(event);
            }
        }
    }

    pub(super) fn notice(&self, key: &SignalKey, event: BleEvent) {
        let mut state = self.reservations.lock().unwrap();
        if let Some(slot) = state.signals.get_mut(key) {
            if let Some(permit) = slot.permit.take().or_else(|| self.slot().ok()) {
                permit.send(event);
                slot.pending = None;
                slot.permit = self.slot().ok();
            } else {
                // Repeated unsolicited state changes coalesce to the latest
                // authoritative state. Request outcomes never use this path.
                slot.pending = Some(event);
            }
        }
    }

    pub(super) fn data(&self, event: BleEvent) -> Result<(), Box<BleEvent>> {
        self.sender
            .try_send(event)
            .map_err(|error| Box::new(error.into_inner()))
    }

    pub(super) fn scan(&self, event: BleEvent) -> bool {
        let mut state = self.reservations.lock().unwrap();
        if state.scans.is_empty() {
            match self.sender.try_send(event) {
                Ok(()) => return false,
                Err(error) => return Self::queue_scan(&mut state, error.into_inner()),
            }
        }
        Self::queue_scan(&mut state, event)
    }

    fn queue_scan(state: &mut Reservations, event: BleEvent) -> bool {
        if let Some(previous) = state.scans.iter_mut().find(|previous| {
            previous.operation_id == event.operation_id
                && previous.context.device_address == event.context.device_address
        }) {
            *previous = event;
            return false;
        }
        let overflow = state.scans.len() >= INTERNAL_CHANNEL_CAPACITY;
        if overflow {
            state.scans.pop_front();
        }
        state.scans.push_back(event);
        overflow
    }

    pub(super) fn flush(&self) {
        let mut state = self.reservations.lock().unwrap();
        for slot in state.signals.values_mut() {
            if let Some(event) = slot.pending.take() {
                match self.sender.try_send(event) {
                    Ok(()) => {}
                    Err(error) => slot.pending = Some(error.into_inner()),
                }
            }
            if !slot.retired && slot.permit.is_none() && slot.pending.is_none() {
                slot.permit = self.slot().ok();
            }
        }
        state
            .signals
            .retain(|_, slot| !slot.retired || slot.pending.is_some());
        while let Some(event) = state.scans.pop_front() {
            if let Err(error) = self.sender.try_send(event) {
                state.scans.push_front(error.into_inner());
                break;
            }
        }
    }

    pub(super) fn clear(&self) {
        *self.reservations.lock().unwrap() = Reservations::default();
    }

    pub(super) fn notification_epoch(&self, device: &DeviceId, generation: u64) {
        self.reservations
            .lock()
            .unwrap()
            .notification_generations
            .insert(device.clone(), generation);
    }

    pub(super) fn is_current(&self, event: &BleEvent) -> bool {
        let state = self.reservations.lock().unwrap();
        if event
            .discovery_generation
            .is_some_and(|generation| generation != state.discovery_generation)
        {
            return false;
        }
        let Some(generation) = event.notification_generation else {
            return true;
        };
        let Ok(device) = DeviceId::parse(&event.context.device_address) else {
            return false;
        };
        state.notification_generations.get(&device) == Some(&generation)
    }

    pub(super) fn discovery_epoch(&self, generation: u64) {
        let mut state = self.reservations.lock().unwrap();
        state.discovery_generation = generation;
        state.scans.clear();
    }
}
