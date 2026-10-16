use super::*;

const SUBSCRIBE_BUFFER_MESSAGES: usize = 64;
const SUBSCRIBE_BUFFER_BYTES: usize = 64 * 1024;

impl CoreState {
    pub(super) async fn ensure_notification_pump(
        &mut self,
        device: &DeviceId,
        tasks: &mut JoinSet<()>,
    ) -> Result<(), BleError> {
        let session = self.devices.get_mut(device).expect("device session");
        if session.notification_cancellation.is_some() {
            return Ok(());
        }
        self.outbox
            .reserve_signal(SignalKey::Notification(device.clone()))?;
        session.notification_generation = session.notification_generation.wrapping_add(1);
        let generation = session.notification_generation;
        self.outbox.notification_epoch(device, generation);
        let cancellation = CancellationToken::new();
        session.notification_cancellation = Some(cancellation.clone());
        session.notification_ready = false;
        let backend = self.backend.clone();
        let internal_tx = self.internal_tx.clone();
        let data = self.data.clone();
        let routes = session.notification_routes.clone();
        let device = device.clone();
        let deadline = self.options.operation_timeout;
        tasks.spawn(async move {
            let result = tokio::select! {
                () = cancellation.cancelled() => return,
                result = guarded("notifications", deadline, backend.notifications(&device)) => result,
            };
            let mut stream = match result {
                Ok(stream) => stream,
                Err(error) => {
                    let _ = internal_tx.send(InternalEvent::NotificationPumpFailed { device, generation, error }).await;
                    return;
                }
            };
            if internal_tx.send(InternalEvent::NotificationReady { device: device.clone(), generation }).await.is_err() { return; }
            loop {
                let next = tokio::select! {
                    () = cancellation.cancelled() => return,
                    next = AssertUnwindSafe(stream.next()).catch_unwind() => next,
                };
                let error = match next {
                    Ok(Some(Ok(notification))) => {
                        let route = routes.lock().unwrap().get(&notification.characteristic_uuid.to_ascii_lowercase()).copied();
                        if route.is_none() { continue; }
                        if data.push(InternalEvent::Notification { device: device.clone(), generation, route, notification }).is_ok() { continue; }
                        BleError::EventOverflow("internal_notifications".into())
                    }
                    Ok(Some(Err(error))) => error,
                    Err(_) => BleError::InternalError("notification stream panicked".into()),
                    Ok(None) => {
                        let _ = internal_tx.send(InternalEvent::NotificationPumpEnded { device, generation }).await;
                        return;
                    }
                };
                let _ = internal_tx.send(InternalEvent::NotificationPumpFailed { device, generation, error }).await;
                return;
            }
        });
        Ok(())
    }

    pub(super) fn start_notification_cleanup(
        &mut self,
        device: &DeviceId,
        tasks: &mut JoinSet<()>,
    ) {
        let session = self.devices.get_mut(device).expect("device session");
        if !session.notification_cleanup || session.cleanup_running || session.in_flight.is_some() {
            return;
        }
        session.cleanup_running = true;
        let keys = std::mem::take(&mut session.cleanup_keys);
        let epoch = session.connection_epoch;
        let backend = self.backend.clone();
        let sender = self.internal_tx.clone();
        let device = device.clone();
        let budget = self.options.shutdown_timeout;
        tasks.spawn(async move {
            let _ = guarded("notification cleanup", budget, async {
                for key in keys {
                    let _ = backend.unsubscribe(&device, &key).await;
                }
                Ok(())
            })
            .await;
            let _ = sender
                .send(InternalEvent::NotificationCleanup { device, epoch })
                .await;
        });
    }

    pub(super) async fn stop_notifications(
        &mut self,
        device: &DeviceId,
        error: BleError,
        tasks: &mut JoinSet<()>,
    ) {
        let Some(session) = self.devices.get_mut(device) else {
            return;
        };
        let had_pump = session.notification_cancellation.is_some();
        let awaiting_pump = had_pump && !session.notification_ready;
        if let Some(token) = session.notification_cancellation.take() {
            token.cancel();
        }
        session.notification_generation = session.notification_generation.wrapping_add(1);
        self.outbox
            .notification_epoch(device, session.notification_generation);
        session.notification_ready = false;
        session.notification_routes.lock().unwrap().clear();
        session.notification_buffer.clear();
        session.notification_buffer_bytes = 0;
        session.cleanup_keys.extend(session.subscriptions.drain());
        if let Some(key) = session.subscribing.take() {
            session.cleanup_keys.insert(key);
        }
        let mut cancelled = None;
        if session
            .in_flight
            .as_ref()
            .is_some_and(|operation| matches!(operation.action, DeviceAction::Subscribe(_)))
        {
            let operation = session.in_flight.take().unwrap();
            operation.abort_handle.abort();
            cancelled = Some((operation.operation_id, operation.action));
        } else if awaiting_pump
            && session
                .pending
                .front()
                .is_some_and(|operation| matches!(operation.action, DeviceAction::Subscribe(_)))
        {
            let operation = session.pending.pop_front().unwrap();
            cancelled = Some((operation.operation_id, operation.action));
        }
        session.notification_cleanup = true;
        self.data.discard_notifications(device);
        if let Some((id, action)) = cancelled {
            self.emit(BleEvent::failed(
                action.operation(),
                id,
                action.context(device),
                error.clone(),
            ))
            .await;
        }
        if had_pump {
            let mut event = BleEvent::failed(
                Operation::Notification,
                OperationId::UNSOLICITED,
                EventContext::for_device(device),
                error,
            );
            event.terminal = false;
            self.emit(event).await;
            self.outbox
                .retire_signal(&SignalKey::Notification(device.clone()));
        }
        self.start_notification_cleanup(device, tasks);
    }

    pub(super) async fn notification(
        &mut self,
        device: DeviceId,
        generation: u64,
        route: Option<OperationId>,
        notification: BackendNotification,
        tasks: &mut JoinSet<()>,
    ) {
        let Some(session) = self.devices.get_mut(&device) else {
            return;
        };
        if session.notification_generation != generation
            || session.state != ConnectionState::Connected
            || session.notification_cancellation.is_none()
        {
            return;
        }
        if route.is_none()
            || session
                .notification_routes
                .lock()
                .unwrap()
                .get(&notification.characteristic_uuid.to_ascii_lowercase())
                .copied()
                != route
        {
            return;
        }
        if session.subscribing.as_ref().is_some_and(|key| {
            key.characteristic_uuid()
                .eq_ignore_ascii_case(&notification.characteristic_uuid)
        }) {
            if session.notification_buffer.len() >= SUBSCRIBE_BUFFER_MESSAGES
                || session
                    .notification_buffer_bytes
                    .saturating_add(notification.value.len())
                    > SUBSCRIBE_BUFFER_BYTES
            {
                self.stop_notifications(
                    &device,
                    BleError::EventOverflow("subscribe_buffer".into()),
                    tasks,
                )
                .await;
            } else {
                session.notification_buffer_bytes += notification.value.len();
                session.notification_buffer.push_back(notification);
            }
            return;
        }
        let Some(key) = session.subscriptions.iter().find(|key| {
            key.characteristic_uuid()
                .eq_ignore_ascii_case(&notification.characteristic_uuid)
        }) else {
            return;
        };
        let mut event = BleEvent::received(
            Operation::Notification,
            EventContext::for_gatt(&device, key),
            EventData::Bytes(notification.value),
        );
        event.connection_state = Some(session.state);
        event.notification_generation = Some(generation);
        if self.outbox.data(event).is_err() {
            self.stop_notifications(
                &device,
                BleError::EventOverflow("outbound_notifications".into()),
                tasks,
            )
            .await;
        }
    }
}
