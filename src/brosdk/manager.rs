//! High-level brosdk lifecycle manager.
//! Wraps the raw FFI into safe Rust and forwards SDK callbacks as Tauri events.

use super::ffi::{BrosdkLib, SdkHandleT};
use once_cell::sync::OnceCell;
use serde::Serialize;
use std::ffi::{c_char, c_void, CStr, CString};
use tracing::{info, warn};

#[cfg(feature = "tauri-app")]
use std::sync::Mutex;

#[cfg(feature = "tauri-app")]
use tauri::{AppHandle, Emitter};

static SDK: OnceCell<BrosdkLib> = OnceCell::new();

#[cfg(feature = "tauri-app")]
static APP_HANDLE: OnceCell<Mutex<AppHandle>> = OnceCell::new();

#[derive(Clone, Serialize)]
pub struct SdkEvent {
    pub code: i32,
    pub data: String,
}

/// Result callback — fired by brosdk for async operations.
unsafe extern "C" fn result_callback(
    code: i32,
    _user_data: *mut c_void,
    data: *const c_char,
    len: usize,
) {
    let payload = if data.is_null() || len == 0 {
        String::new()
    } else {
        let slice = std::slice::from_raw_parts(data as *const u8, len);
        String::from_utf8_lossy(slice).into_owned()
    };

    info!("brosdk callback: code={code}, len={len}");
    info!("brosdk callback data: {payload}");

    #[cfg(feature = "tauri-app")]
    {
        if let Some(handle) = APP_HANDLE.get() {
            if let Ok(h) = handle.lock() {
                let _ = h.emit("brosdk-event", SdkEvent { code, data: payload });
            }
        }
    }
}

/// Cookies/storage callback — pass-through using SDK's own allocator to avoid cross-allocator free.
unsafe extern "C" fn cookies_storage_callback(
    data: *const c_char,
    len: usize,
    new_data: *mut *mut c_char,
    new_len: *mut usize,
    _user_data: *mut c_void,
) {
    if data.is_null() || len == 0 {
        return;
    }
    // Must use sdk_malloc so the SDK can safely call sdk_free on the returned buffer.
    let sdk = match SDK.get() {
        Some(s) => s,
        None => return,
    };
    let buf = (sdk.sdk_malloc)(len);
    if !buf.is_null() {
        std::ptr::copy_nonoverlapping(data as *const u8, buf as *mut u8, len);
        *new_data = buf as *mut c_char;
        *new_len = len;
    }
}

/// Load and initialize the brosdk library (with Tauri).
#[cfg(feature = "tauri-app")]
pub fn load(app: AppHandle, lib_path: &str) -> Result<(), String> {
    APP_HANDLE
        .set(Mutex::new(app))
        .map_err(|_| "APP_HANDLE already set")?;

    let lib = unsafe { BrosdkLib::load(lib_path)? };
    SDK.set(lib).map_err(|_| "SDK already loaded")?;

    register_callbacks();
    info!("brosdk loaded from {lib_path}");
    Ok(())
}

/// Load and initialize the brosdk library (without Tauri).
#[cfg(not(feature = "tauri-app"))]
pub fn load(lib_path: &str) -> Result<(), String> {
    let lib = unsafe { BrosdkLib::load(lib_path)? };
    SDK.set(lib).map_err(|_| "SDK already loaded")?;

    register_callbacks();
    info!("brosdk loaded from {lib_path}");
    Ok(())
}

fn register_callbacks() {
    let sdk = SDK.get().unwrap();
    unsafe {
        let rc = (sdk.sdk_register_result_cb)(result_callback, std::ptr::null_mut());
        if !(sdk.sdk_is_ok)(rc) {
            warn!("sdk_register_result_cb returned {rc}");
        }

        let rc = (sdk.sdk_register_cookies_storage_cb)(
            cookies_storage_callback,
            std::ptr::null_mut(),
        );
        if !(sdk.sdk_is_ok)(rc) {
            warn!("sdk_register_cookies_storage_cb returned {rc}");
        }
    }
}

/// Initialize the SDK with user credentials.
pub fn init(user_sig: &str, work_dir: &str, port: u16) -> Result<String, String> {
    let sdk = SDK.get().ok_or("SDK not loaded")?;

    let init_json = serde_json::json!({
        "userSig": user_sig,
        "workDir": work_dir,
        "port": port,
    });
    let data = CString::new(init_json.to_string()).map_err(|e| e.to_string())?;
    let data_bytes = data.as_bytes();

    let mut handle: SdkHandleT = std::ptr::null_mut();
    let mut out_data: *mut c_char = std::ptr::null_mut();
    let mut out_len: usize = 0;

    let code = unsafe {
        (sdk.sdk_init)(
            &mut handle,
            data_bytes.as_ptr() as *const c_char,
            data_bytes.len(),
            &mut out_data,
            &mut out_len,
        )
    };

    if unsafe { (sdk.sdk_is_error)(code) } {
        let err = unsafe {
            let ptr = (sdk.sdk_error_string)(code);
            if ptr.is_null() {
                format!("SDK init error: {code}")
            } else {
                CStr::from_ptr(ptr).to_string_lossy().to_string()
            }
        };
        return Err(err);
    }

    let result = unsafe { sdk.take_string(out_data, out_len) };
    let result = if result.is_empty() { "{}".to_string() } else { result };
    info!("brosdk initialized, port={port}, result={result}");
    Ok(result)
}

/// Open a browser with the given env ID.
/// SDK expects: {"envs": [{"envId": "...", "args": [...]}]}
pub fn browser_open(env_id: &str) -> Result<(), String> {
    let sdk = SDK.get().ok_or("SDK not loaded")?;

    let config = serde_json::json!({
        "envs": [{
            "envId": env_id,
            "args": ["--no-first-run", "--no-default-browser-check", "--remote-debugging-port=9222"],
        }]
    });
    let json = config.to_string();
    info!("browser_open request: {json}");

    let data = json.as_bytes();
    let code = unsafe { (sdk.sdk_browser_open)(data.as_ptr() as *const c_char, data.len()) };

    let is_ok    = unsafe { (sdk.sdk_is_ok)(code) };
    let is_done  = unsafe { (sdk.sdk_is_done)(code) };
    let is_error = unsafe { (sdk.sdk_is_error)(code) };
    let is_reqid = unsafe { (sdk.sdk_is_reqid)(code) };
    info!("browser_open code={code} ok={is_ok} done={is_done} error={is_error} reqid={is_reqid}");

    if is_error {
        // 获取 SDK 返回的原始结果
        let out_data: *mut c_char = std::ptr::null_mut();
        let out_len: usize = 0;
        let result = unsafe { sdk.take_string(out_data, out_len) };
        
        let err = unsafe {
            let ptr = (sdk.sdk_error_string)(code);
            if ptr.is_null() { format!("browser_open error: {code}") }
            else { CStr::from_ptr(ptr).to_string_lossy().to_string() }
        };
        
        if result.is_empty() {
            return Err(err);
        } else {
            return Err(format!("{} | result: {}", err, result));
        }
    }
    Ok(())
}

/// Close a browser.
/// SDK expects: {"envs": ["envId1", "envId2", ...]}
pub fn browser_close(env_id: &str) -> Result<(), String> {
    let sdk = SDK.get().ok_or("SDK not loaded")?;

    let config = serde_json::json!({ "envs": [env_id] });
    let json = config.to_string();
    info!("browser_close request: {json}");

    let data = json.as_bytes();
    let code = unsafe { (sdk.sdk_browser_close)(data.as_ptr() as *const c_char, data.len()) };

    if unsafe { (sdk.sdk_is_error)(code) } {
        // 获取 SDK 返回的原始结果
        let out_data: *mut c_char = std::ptr::null_mut();
        let out_len: usize = 0;
        let result = unsafe { sdk.take_string(out_data, out_len) };
        
        let err = unsafe {
            let ptr = (sdk.sdk_error_string)(code);
            if ptr.is_null() { format!("browser_close error: {code}") }
            else { CStr::from_ptr(ptr).to_string_lossy().to_string() }
        };
        
        if result.is_empty() {
            return Err(err);
        } else {
            return Err(format!("{} | result: {}", err, result));
        }
    }
    Ok(())
}

/// Update the access token.
pub fn token_update(token_json: &str) -> Result<(), String> {
    let sdk = SDK.get().ok_or("SDK not loaded")?;
    let data = token_json.as_bytes();
    let code = unsafe { (sdk.sdk_token_update)(data.as_ptr() as *const c_char, data.len()) };
    if unsafe { (sdk.sdk_is_error)(code) } {
        let err = unsafe {
            let ptr = (sdk.sdk_error_string)(code);
            if ptr.is_null() { format!("token_update error: {code}") }
            else { CStr::from_ptr(ptr).to_string_lossy().to_string() }
        };
        return Err(err);
    }
    Ok(())
}

/// Query SDK runtime information (version, state, etc.).
///
/// Wraps `sdk_info(char **out_data, size_t *out_len)`:
/// - No input parameters — pure query call.
/// - SDK allocates the output buffer; `take_string` frees it via `sdk_free`.
/// - Returns a JSON string on success, e.g. `{"version":"1.2.3","state":"ready"}`.
pub fn sdk_info() -> Result<String, String> {
    let sdk = SDK.get().ok_or("SDK not loaded")?;

    let mut out_data: *mut c_char = std::ptr::null_mut();
    let mut out_len: usize = 0;

    let code = unsafe { (sdk.sdk_info)(&mut out_data, &mut out_len) };

    if unsafe { (sdk.sdk_is_error)(code) } {
        let err = unsafe {
            let ptr = (sdk.sdk_error_string)(code);
            if ptr.is_null() {
                format!("sdk_info error: {code}")
            } else {
                CStr::from_ptr(ptr).to_string_lossy().to_string()
            }
        };
        return Err(err);
    }

    let result = unsafe { sdk.take_string(out_data, out_len) };
    let result = if result.is_empty() {
        "{}".to_string()
    } else {
        result
    };
    info!("sdk_info result: {result}");
    Ok(result)
}

/// Create a new environment.
///
/// Wraps `sdk_env_create(const char *data, size_t len, char **out_data, size_t *out_len)`:
/// - Input: JSON with env config, e.g. `{"kernelVersion":"120"}` or `{"envId":"...","args":["..."]}`
/// - Output: JSON with created env info; caller must free via sdk_free.
/// - Returns a JSON string on success, e.g. `{"envId":"...","envName":"...","finger":{"kernelVersion":"120"}}`.
pub fn sdk_env_create(config_json: &str) -> Result<String, String> {
    let sdk = SDK.get().ok_or("SDK not loaded")?;

    let data = config_json.as_bytes();
    let mut out_data: *mut c_char = std::ptr::null_mut();
    let mut out_len: usize = 0;

    let code = unsafe {
        (sdk.sdk_env_create)(
            data.as_ptr() as *const c_char,
            data.len(),
            &mut out_data,
            &mut out_len,
        )
    };

    if unsafe { (sdk.sdk_is_error)(code) } {
        // 获取 SDK 返回的原始结果（可能包含错误详情）
        let result = unsafe { sdk.take_string(out_data, out_len) };
        
        let err = unsafe {
            let ptr = (sdk.sdk_error_string)(code);
            if ptr.is_null() {
                format!("sdk_env_create error: {code}")
            } else {
                CStr::from_ptr(ptr).to_string_lossy().to_string()
            }
        };
        // 返回更详细的错误信息：SDK错误 + 原始返回
        if result.is_empty() {
            return Err(err);
        } else {
            return Err(format!("{} | result: {}", err, result));
        }
    }

    let result = unsafe { sdk.take_string(out_data, out_len) };
    let result = if result.is_empty() {
        "{}".to_string()
    } else {
        result
    };
    info!("sdk_env_create result: {result}");
    Ok(result)
}

/// Query environment list with pagination.
///
/// Wraps `sdk_env_page(const char *data, size_t len, char **out_data, size_t *out_len)`:
/// - Input: JSON with pagination params, e.g. `{"page":1,"pageSize":10}` (can be empty `{}`)
/// - Output: JSON with environment list; caller must free via sdk_free.
/// - Returns a JSON string on success, e.g. `{"list":[{"envId":"...","envName":"..."}],"total":10}`.
pub fn sdk_env_page(page_json: &str) -> Result<String, String> {
    let sdk = SDK.get().ok_or("SDK not loaded")?;

    let data = page_json.as_bytes();
    let mut out_data: *mut c_char = std::ptr::null_mut();
    let mut out_len: usize = 0;

    let code = unsafe {
        (sdk.sdk_env_page)(
            data.as_ptr() as *const c_char,
            data.len(),
            &mut out_data,
            &mut out_len,
        )
    };

    if unsafe { (sdk.sdk_is_error)(code) } {
        let err = unsafe {
            let ptr = (sdk.sdk_error_string)(code);
            if ptr.is_null() {
                format!("sdk_env_page error: {code}")
            } else {
                CStr::from_ptr(ptr).to_string_lossy().to_string()
            }
        };
        return Err(err);
    }

    let result = unsafe { sdk.take_string(out_data, out_len) };
    let result = if result.is_empty() {
        "{}".to_string()
    } else {
        result
    };
    info!("sdk_env_page result: {result}");
    Ok(result)
}

/// Shutdown the SDK gracefully.
pub fn shutdown() -> Result<(), String> {
    let sdk = SDK.get().ok_or("SDK not loaded")?;
    let code = unsafe { (sdk.sdk_shutdown)() };
    if unsafe { (sdk.sdk_is_error)(code) } {
        return Err(format!("shutdown error: {code}"));
    }
    info!("brosdk shutdown complete");
    Ok(())
}
