//! High-rate data is isolated from the coordinator's reliable control channel.
use super::*;
use std::sync::Mutex;
use tokio::sync::Notify;

#[derive(Default)]
struct DataQueue {
    events: VecDeque<InternalEvent>,
    overflow_reported: bool,
}

#[derive(Default)]
pub(super) struct DataInbox {
    queue: Mutex<DataQueue>,
    changed: Notify,
}

impl DataInbox {
    // Ok(true) reports the first advertisement eviction in an overload episode.
    // Notifications are never silently evicted: their pump must fail explicitly.
    pub(super) fn push(&self, event: InternalEvent) -> Result<bool, Box<InternalEvent>> {
        let mut queue = self.queue.lock().unwrap();
        if let InternalEvent::Backend {
            generation,
            result: Ok(BackendEvent::Changed { device, update }),
        } = &event
        {
            if let Some(previous) = queue.events.iter_mut().find(|previous| matches!(previous,
                InternalEvent::Backend { generation: old_generation, result: Ok(BackendEvent::Changed { device: old_device, update: old_update }) }
                if old_generation == generation && old_device == device && std::mem::discriminant(old_update) == std::mem::discriminant(update))) {
                *previous = event;
                return Ok(false);
            }
        }
        let mut overflow = false;
        if queue.events.len() == INTERNAL_CHANNEL_CAPACITY {
            if matches!(event, InternalEvent::Notification { .. }) {
                return Err(Box::new(event));
            }
            overflow = !queue.overflow_reported;
            queue.overflow_reported = true;
            if let Some(index) = queue
                .events
                .iter()
                .position(|event| !matches!(event, InternalEvent::Notification { .. }))
            {
                queue.events.remove(index);
            } else {
                return Ok(overflow);
            }
        }
        queue.events.push_back(event);
        drop(queue);
        self.changed.notify_one();
        Ok(overflow)
    }

    pub(super) async fn recv(&self) -> InternalEvent {
        loop {
            let notified = self.changed.notified();
            {
                let mut queue = self.queue.lock().unwrap();
                if let Some(event) = queue.events.pop_front() {
                    if queue.events.len() < INTERNAL_CHANNEL_CAPACITY / 2 {
                        queue.overflow_reported = false;
                    }
                    return event;
                }
            }
            notified.await;
        }
    }

    pub(super) fn discard_notifications(&self, device: &DeviceId) {
        self.queue.lock().unwrap().events.retain(|event|
            !matches!(event, InternalEvent::Notification { device: source, .. } if source == device));
    }

    pub(super) fn discard_advertisements(&self) {
        self.queue
            .lock()
            .unwrap()
            .events
            .retain(|event| matches!(event, InternalEvent::Notification { .. }));
    }
}
