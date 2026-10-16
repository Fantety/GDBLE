use super::*;

impl CoreState {
    pub(super) async fn scan_overflow(&self, source: &str) {
        let mut event = BleEvent::failed(
            Operation::Scan,
            OperationId::UNSOLICITED,
            EventContext::default(),
            BleError::EventOverflow(source.into()),
        );
        event.terminal = false;
        self.emit(event).await;
    }

    pub(super) async fn advertisement(
        &mut self,
        device: DeviceId,
        update: AdvertisementUpdate,
        tasks: &mut JoinSet<()>,
    ) {
        if !self.initialized {
            return;
        }
        if update != AdvertisementUpdate::Refresh {
            if let Some(mut info) = self.known_devices.get(&device).cloned() {
                update.apply(&mut info);
                self.device_progress(info).await;
            }
        }
        if let Some(pending) = self.snapshots.get_mut(&device) {
            if let Some(old) = pending
                .updates
                .iter_mut()
                .find(|old| std::mem::discriminant(*old) == std::mem::discriminant(&update))
            {
                *old = update;
            } else {
                pending.updates.push(update);
            }
            return;
        }
        if self.snapshots.len() >= COMMAND_CHANNEL_CAPACITY {
            self.scan_overflow("snapshot_requests").await;
            return;
        }
        self.snapshots.insert(
            device.clone(),
            PendingSnapshot {
                updates: vec![update],
            },
        );
        let backend = self.backend.clone();
        let sender = self.internal_tx.clone();
        let generation = self.work_generation;
        let deadline = self.options.operation_timeout;
        tasks.spawn(async move {
            let result = guarded("device snapshot", deadline, backend.device_info(&device)).await;
            let _ = sender
                .send(InternalEvent::Snapshot {
                    device,
                    generation,
                    result,
                })
                .await;
        });
    }

    pub(super) async fn snapshot_completed(
        &mut self,
        device: DeviceId,
        generation: u64,
        result: Result<DeviceInfo, BleError>,
    ) {
        if generation != self.work_generation || !self.initialized {
            return;
        }
        let Some(pending) = self.snapshots.remove(&device) else {
            return;
        };
        match result {
            Ok(mut info) => {
                for update in pending.updates {
                    update.apply(&mut info);
                }
                self.device_progress(info).await;
            }
            Err(error) => {
                let mut event = BleEvent::failed(
                    Operation::Scan,
                    OperationId::UNSOLICITED,
                    EventContext::for_device(&device),
                    error,
                );
                event.terminal = false;
                self.emit(event).await;
            }
        }
    }

    pub(super) async fn adapter_changed(
        &mut self,
        availability: AdapterState,
        tasks: &mut JoinSet<()>,
    ) {
        if self.adapter_state == availability {
            return;
        }
        self.adapter_state = availability;
        if self.adapter_state != AdapterState::PoweredOn {
            self.invalidate_adapter(
                BleError::AdapterUnavailable(self.adapter_state.as_str().into()),
                false,
                tasks,
            )
            .await;
        } else {
            let mut event = BleEvent::received(
                Operation::Initialize,
                EventContext::default(),
                EventData::Reason("powered_on".into()),
            );
            event.phase = Phase::Progress;
            self.emit(event).await;
        }
    }

    pub(super) async fn remote_disconnect(&mut self, device: DeviceId, tasks: &mut JoinSet<()>) {
        let Some(session) = self.devices.get_mut(&device) else {
            return;
        };
        match session.state {
            ConnectionState::Disconnected => return,
            ConnectionState::Disconnecting => {
                session.local_disconnect_observed = true;
                return;
            }
            ConnectionState::Connecting if session.has_local_disconnect => {
                session.suspect_disconnect = true;
                return;
            }
            ConnectionState::Connected if session.has_local_disconnect => {
                self.check_connection(&device, tasks);
                return;
            }
            ConnectionState::Connecting | ConnectionState::Connected => {}
        }
        self.invalidate_device(device, tasks).await;
    }

    pub(super) fn check_connection(&mut self, device: &DeviceId, tasks: &mut JoinSet<()>) {
        let session = self.devices.get_mut(device).expect("device session");
        if session.checking_connection {
            return;
        }
        session.checking_connection = true;
        let epoch = session.connection_epoch;
        let backend = self.backend.clone();
        let sender = self.internal_tx.clone();
        let device = device.clone();
        let deadline = self.options.operation_timeout;
        tasks.spawn(async move {
            let result = guarded("connection state", deadline, backend.is_connected(&device)).await;
            let _ = sender
                .send(InternalEvent::ConnectionChecked {
                    device,
                    epoch,
                    result,
                })
                .await;
        });
    }

    pub(super) async fn connection_checked(
        &mut self,
        device: DeviceId,
        epoch: u64,
        result: Result<bool, BleError>,
        tasks: &mut JoinSet<()>,
    ) {
        let Some(session) = self.devices.get_mut(&device) else {
            return;
        };
        if session.connection_epoch != epoch || !session.checking_connection {
            return;
        }
        session.checking_connection = false;
        session.suspect_disconnect = false;
        let completion = session.connection_completion.take();
        match result {
            Ok(true) => {
                if let Some((id, action, result)) = completion {
                    let generation = session
                        .in_flight
                        .as_ref()
                        .map(|operation| operation.generation);
                    if let Some(generation) = generation {
                        self.device_completed(
                            id,
                            device.clone(),
                            generation,
                            action,
                            result,
                            tasks,
                        )
                        .await;
                    }
                } else {
                    self.start_next_device_action(&device, tasks).await;
                }
            }
            result => {
                let error = result.err();
                if let Some(error) = &error {
                    self.devices.get_mut(&device).unwrap().state = ConnectionState::Disconnected;
                    let mut event = BleEvent::failed(
                        Operation::Disconnect,
                        OperationId::UNSOLICITED,
                        EventContext::for_device(&device),
                        error.clone(),
                    );
                    event.terminal = false;
                    self.emit(event).await;
                }
                self.invalidate_device(device.clone(), tasks).await;
                if error.is_some() {
                    // Serialize recovery with new commands for this device.
                    let session = self.devices.get_mut(&device).unwrap();
                    session.notification_cleanup = true;
                    session.cleanup_running = true;
                    let epoch = session.connection_epoch;
                    let backend = self.backend.clone();
                    let sender = self.internal_tx.clone();
                    let budget = self.options.shutdown_timeout;
                    tasks.spawn(async move {
                        let _ = guarded(
                            "unverified connection cleanup",
                            budget,
                            backend.disconnect(&device),
                        )
                        .await;
                        let _ = sender
                            .send(InternalEvent::NotificationCleanup { device, epoch })
                            .await;
                    });
                }
            }
        }
    }

    pub(super) async fn services_changed(&mut self, device: DeviceId, tasks: &mut JoinSet<()>) {
        let Some(session) = self.devices.get_mut(&device) else {
            return;
        };
        if session.state != ConnectionState::Connected {
            return;
        }
        session.services.clear();
        session.services_invalidated = true;
        let mut cancelled = Vec::new();
        if session.in_flight.as_ref().is_some_and(|operation| {
            !matches!(
                operation.action,
                DeviceAction::Connect | DeviceAction::Disconnect
            )
        }) {
            let operation = session.in_flight.take().unwrap();
            operation.abort_handle.abort();
            cancelled.push((operation.operation_id, operation.action));
        }
        session.pending.retain(|operation| {
            if matches!(
                operation.action,
                DeviceAction::Connect | DeviceAction::Disconnect | DeviceAction::DiscoverServices
            ) {
                true
            } else {
                cancelled.push((operation.operation_id, operation.action.clone()));
                false
            }
        });
        for (id, action) in cancelled {
            self.emit(BleEvent::failed(
                action.operation(),
                id,
                action.context(&device),
                BleError::ServicesChanged,
            ))
            .await;
        }
        let mut event = BleEvent::failed(
            Operation::DiscoverServices,
            OperationId::UNSOLICITED,
            EventContext::for_device(&device),
            BleError::ServicesChanged,
        );
        event.terminal = false;
        self.emit(event).await;
        self.stop_notifications(&device, BleError::ServicesChanged, tasks)
            .await;
        self.start_next_device_action(&device, tasks).await;
    }
}
