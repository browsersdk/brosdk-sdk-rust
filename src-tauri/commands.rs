//! Tauri commands for Brosdk SDK Demo

use brosdk_sdk::brosdk::manager;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::{AppHandle, State};

const GET_USER_SIG_URL: &str = "https://api.brosdk.com/api/v2/browser/getUserSig";
const CREATE_ENV_URL: &str = "https://api.brosdk.com/api/v2/browser/create";

pub struct AppState {
    pub api_key: Mutex<String>,
    pub initialized: Mutex<bool>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            api_key: Mutex::new(String::new()),
            initialized: Mutex::new(false),
        }
    }
}

#[derive(Serialize)]
struct GetUserSigRequest {
    #[serde(rename = "customerId")]
    customer_id: String,
    duration: u64,
}

#[derive(Deserialize)]
struct GetUserSigData {
    #[serde(rename = "userSig")]
    user_sig: String,
}

#[derive(Deserialize)]
struct GetUserSigResponse {
    code: i32,
    msg: String,
    data: Option<GetUserSigData>,
}

/// 使用 API Key 换取 userSig
async fn fetch_user_sig(api_key: &str) -> Result<String, String> {
    let client = reqwest::Client::new();

    let body = GetUserSigRequest {
        customer_id: "default".to_string(),
        duration: 2592000,
    };

    let resp = client
        .post(GET_USER_SIG_URL)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Accept", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("请求失败: {}", e))?;

    let result: GetUserSigResponse = resp
        .json()
        .await
        .map_err(|e| format!("解析响应失败: {}", e))?;

    if result.code != 200 {
        return Err(format!("获取 userSig 失败: {} (code={})", result.msg, result.code));
    }

    result
        .data
        .map(|d| d.user_sig)
        .ok_or_else(|| "响应中缺少 data 字段".to_string())
}

// ── create env ──────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct FingerConfig {
    kernel: String,
    #[serde(rename = "kernelVersion")]
    kernel_version: String,
    system: String,
    #[serde(rename = "publicIp")]
    public_ip: String,
}

#[derive(Serialize)]
struct CreateEnvRequest {
    #[serde(rename = "customerId")]
    customer_id: String,
    #[serde(rename = "deviceName")]
    device_name: String,
    #[serde(rename = "envName")]
    env_name: String,
    finger: FingerConfig,
}

#[derive(Deserialize)]
struct CreateEnvData {
    #[serde(rename = "envId")]
    env_id: String,
    #[serde(rename = "envName")]
    env_name: String,
}

#[derive(Deserialize)]
struct CreateEnvResponse {
    code: i32,
    msg: String,
    data: Option<CreateEnvData>,
}

async fn api_create_env(api_key: &str, kernel_version: &str) -> Result<CreateEnvData, String> {
    let client = reqwest::Client::new();

    let body = CreateEnvRequest {
        customer_id: "default".to_string(),
        device_name: "brosdk-demo".to_string(),
        env_name: format!("env-{}", chrono::Utc::now().timestamp()),
        finger: FingerConfig {
            kernel: "Chrome".to_string(),
            kernel_version: kernel_version.to_string(),
            system: "All Windows".to_string(),
            public_ip: "127.0.0.1".to_string(),
        },
    };

    tracing::info!("create env request: {}", serde_json::to_string(&body).unwrap_or_default());

    let resp = client
        .post(CREATE_ENV_URL)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Accept", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("请求失败: {}", e))?;

    let body = resp
        .text()
        .await
        .map_err(|e| format!("读取响应失败: {}", e))?;

    tracing::info!("create env response: {}", body);

    let result: CreateEnvResponse = serde_json::from_str(&body)
        .map_err(|e| format!("解析响应失败: {}", e))?;

    if result.code != 200 {
        return Err(format!("创建环境失败: {} (code={})", result.msg, result.code));
    }

    result.data.ok_or_else(|| "响应中缺少 data 字段".to_string())
}

// ────────────────────────────────────────────────────────────────────────────

/// SDK 初始化：用 apiKey 换取 userSig，再调用 manager::init
#[tauri::command]
pub async fn init_sdk(
    app: AppHandle,
    api_key: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    tracing::info!("Fetching userSig for API key");

    *state.api_key.lock().unwrap() = api_key.clone();

    let user_sig = fetch_user_sig(&api_key).await?;
    tracing::info!("userSig obtained successfully");

    #[cfg(target_os = "windows")]
    let lib_path = "libs/windows-x64/brosdk.dll";

    #[cfg(target_os = "macos")]
    let lib_path = "libs/macos-arm64/brosdk.dylib";

    match manager::load(app.clone(), lib_path) {
        Ok(_) => {
            let work_dir = std::env::temp_dir().to_string_lossy().to_string();
            // let work_dir = std::env::current_dir()
            //     .unwrap_or_else(|_| std::path::PathBuf::from("."))
            //     .to_string_lossy()
            //     .to_string();
            match manager::init(&user_sig, &work_dir, 8080) {
                Ok(result) => {
                    *state.initialized.lock().unwrap() = true;
                    tracing::info!("SDK initialized: {}", result);
                    Ok(format!("SDK 初始化成功: {}", result))
                }
                Err(e) => Err(format!("SDK 初始化失败: {}", e)),
            }
        }
        Err(e) => {
            tracing::warn!("Failed to load SDK library: {}. Using mock mode.", e);
            *state.initialized.lock().unwrap() = true;
            Ok(format!("模拟模式：SDK 初始化成功（userSig 已获取）"))
        }
    }
}

/// 创建环境 — 调用 REST API，返回新建的 envId
#[tauri::command]
pub async fn create_env(
    kernel_version: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    if !*state.initialized.lock().unwrap() {
        return Err("SDK 未初始化".to_string());
    }

    let api_key = state.api_key.lock().unwrap().clone();

    let data = api_create_env(&api_key, &kernel_version).await?;
    tracing::info!("Environment created: {} ({})", data.env_name, data.env_id);
    Ok(data.env_id)
}

/// 启动环境
#[tauri::command]
pub async fn start_env(env_id: String, state: State<'_, AppState>) -> Result<String, String> {
    if !*state.initialized.lock().unwrap() {
        return Err("SDK 未初始化".to_string());
    }

    match manager::browser_open(&env_id) {
        Ok(_) => Ok(format!("环境 {} 启动请求已发送", env_id)),
        Err(e) => Err(e),
    }
}

/// 关闭环境
#[tauri::command]
pub async fn stop_env(env_id: String, state: State<'_, AppState>) -> Result<String, String> {
    if !*state.initialized.lock().unwrap() {
        return Err("SDK 未初始化".to_string());
    }

    match manager::browser_close(&env_id) {
        Ok(_) => Ok(format!("环境 {} 已关闭", env_id)),
        Err(e) => Err(e),
    }
}
