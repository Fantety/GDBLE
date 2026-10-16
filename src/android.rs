use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::{Arc, Mutex, OnceLock};

use futures::StreamExt;
use jni::objects::{GlobalRef, JObject};
use jni::sys::{jboolean, jint, JNI_FALSE, JNI_TRUE};
use jni::{JNIEnv, JavaVM};
use tokio::sync::broadcast;

use crate::backend::{AdapterState, BackendEvent, BackendEventStream};
use crate::types::BleError;

static ANDROID_INITIALIZATION: OnceLock<Result<(), String>> = OnceLock::new();
struct AndroidBridge {
    vm: JavaVM,
    plugin: GlobalRef,
}
static BRIDGE: Mutex<Option<Arc<AndroidBridge>>> = Mutex::new(None);
static ADAPTER_EVENTS: OnceLock<broadcast::Sender<AdapterState>> = OnceLock::new();

fn state_events() -> &'static broadcast::Sender<AdapterState> {
    ADAPTER_EVENTS.get_or_init(|| broadcast::channel(32).0)
}

fn android_state(state: jint) -> AdapterState {
    match state {
        12 => AdapterState::PoweredOn,  // BluetoothAdapter.STATE_ON
        10 => AdapterState::PoweredOff, // BluetoothAdapter.STATE_OFF
        _ => AdapterState::Unknown,
    }
}

/// btleplug 0.12's Android query always returns Unknown. Query the actual
/// BluetoothAdapter through the plugin, after application permission gating.
pub fn adapter_state() -> Result<AdapterState, BleError> {
    ensure_initialized()?;
    let bridge = BRIDGE.lock().unwrap().clone().ok_or_else(|| {
        BleError::AndroidNotInitialized("Android state bridge unavailable".into())
    })?;
    let env = bridge
        .vm
        .attach_current_thread()
        .map_err(|error| BleError::OperationFailed(error.to_string()))?;
    let result = env
        .call_method(bridge.plugin.as_obj(), "queryAdapterState", "()I", &[])
        .and_then(|value| value.i());
    match result {
        Ok(state) => Ok(android_state(state)),
        Err(error) => {
            let _ = env.exception_clear();
            Err(BleError::OperationFailed(format!(
                "Android adapter state: {error}"
            )))
        }
    }
}

fn adapter_events() -> BackendEventStream {
    Box::pin(futures::stream::unfold(
        state_events().subscribe(),
        |mut receiver| async move {
            match receiver.recv().await {
                Ok(state) => Some((Ok(BackendEvent::StateChanged(state)), receiver)),
                // If power transitions were lost, fail closed. Buffered states then
                // restore the latest availability, requiring explicit initialize.
                Err(broadcast::error::RecvError::Lagged(_)) => Some((
                    Ok(BackendEvent::StateChanged(AdapterState::Unknown)),
                    receiver,
                )),
                Err(broadcast::error::RecvError::Closed) => None,
            }
        },
    ))
}

pub fn with_adapter_states(events: BackendEventStream) -> BackendEventStream {
    Box::pin(futures::stream::unfold(
        (events, adapter_events()),
        |(mut events, mut states)| async move {
            // Either stream ending must invalidate the adapter session. select()
            // would hide a dead btleplug stream behind the process-wide bridge.
            let event =
                tokio::select! { event = events.next() => event, state = states.next() => state };
            event.map(|event| (event, (events, states)))
        },
    ))
}

#[no_mangle]
pub extern "system" fn Java_org_gdble_android_GDBLEPlugin_reportAdapterState(
    env: JNIEnv<'_>,
    plugin: JObject<'_>,
    state: jint,
) {
    let mut bridge = BRIDGE.lock().unwrap();
    if bridge.as_ref().is_some_and(|bridge| {
        env.is_same_object(plugin, bridge.plugin.as_obj())
            .unwrap_or(false)
    }) {
        let _ = state_events().send(android_state(state));
        if state == -1 {
            // Plugin destruction releases the GlobalRef, including its Activity
            // ownership. A later plugin instance installs a fresh bridge.
            bridge.take();
        }
    }
}

pub fn ensure_initialized() -> Result<(), BleError> {
    match ANDROID_INITIALIZATION.get() {
        Some(Ok(())) => Ok(()),
        Some(Err(error)) => Err(BleError::AndroidNotInitialized(error.clone())),
        None => Err(BleError::AndroidNotInitialized(
            "GDBLEPlugin.initializeNative() has not been called".to_string(),
        )),
    }
}

#[no_mangle]
pub extern "system" fn Java_org_gdble_android_GDBLEPlugin_initializeNative(
    env: JNIEnv<'_>,
    plugin: JObject<'_>,
) -> jboolean {
    // Activity/plugin recreation replaces only the context bridge. btleplug's
    // process-wide JNI registration remains idempotent.
    let bridge = env.get_java_vm().and_then(|vm| {
        env.new_global_ref(plugin)
            .map(|plugin| Arc::new(AndroidBridge { vm, plugin }))
    });
    match bridge {
        Ok(bridge) => *BRIDGE.lock().unwrap() = Some(bridge),
        Err(error) => {
            let _ = env.exception_clear();
            eprintln!("[GDBLE] Android state bridge initialization failed: {error}");
            return JNI_FALSE;
        }
    }
    let result = ANDROID_INITIALIZATION.get_or_init(|| {
        catch_unwind(AssertUnwindSafe(|| btleplug::platform::init(&env)))
            .map_err(|_| "btleplug Android initialization panicked".to_string())?
            .map_err(|error| error.to_string())
    });
    match result {
        Ok(()) => JNI_TRUE,
        Err(error) => {
            eprintln!("[GDBLE] Android initialization failed: {error}");
            JNI_FALSE
        }
    }
}
