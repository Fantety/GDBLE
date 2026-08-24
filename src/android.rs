use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::OnceLock;

use jni::objects::JObject;
use jni::sys::{jboolean, JNI_FALSE, JNI_TRUE};
use jni::JNIEnv;

use crate::types::BleError;

static ANDROID_INITIALIZATION: OnceLock<Result<(), String>> = OnceLock::new();

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
    _plugin: JObject<'_>,
) -> jboolean {
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
