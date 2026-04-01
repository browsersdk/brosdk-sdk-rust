//! Tauri commands for Brosdk SDK Demo

use brosdk_sdk::brosdk::manager;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::{AppHandle, State};

const GET_USER_SIG_URL: &str = "https://api.brosdk.com/api/v2/browser/getUserSig";
const CREATE_ENV_URL: &str = "https://api.brosdk.com/api/v2/browser/create";
const PAGE_ENV_URL: &str = "https://api.brosdk.com/api/v2/browser/page";

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
    /// 代理配置，格式如：http://127.0.0.1:8080 或 socks5://user:pwd@ipaddr:6666
    #[serde(skip_serializing_if = "Option::is_none")]
    proxy: Option<String>,
    /// 代理桥配置
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "bridgeProxy")]
    bridge_proxy: Option<String>,
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

async fn api_create_env(api_key: &str, kernel_version: &str, proxy: Option<&str>, bridge_proxy: Option<&str>) -> Result<CreateEnvData, String> {
    let client = reqwest::Client::new();

    let proxy_str = proxy.filter(|s| !s.is_empty()).map(|s| s.to_string());
    let bridge_proxy_str = bridge_proxy.filter(|s| !s.is_empty()).map(|s| s.to_string());

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
        proxy: proxy_str,
        bridge_proxy: bridge_proxy_str,
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

// ── list envs ─────────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct PageEnvRequest {
    #[serde(rename = "customerId")]
    customer_id: String,
    page: u32,
    page_size: u32,
}

#[derive(Deserialize, Default)]
struct FingerData {
    #[serde(rename = "kernelVersion", default)]
    kernel_version: Option<String>,
}

#[derive(Deserialize)]
struct FingerEnvItem {
    #[serde(rename = "envId")]
    env_id: String,
    #[serde(rename = "envName")]
    env_name: String,
    #[serde(rename = "finger", default)]
    finger: FingerData,
}

#[derive(Deserialize)]
struct PageEnvData {
    #[serde(default)]
    list: Vec<FingerEnvItem>,
    #[serde(default)]
    total: u32,
}

#[derive(Deserialize)]
struct PageEnvResponse {
    code: i32,
    msg: String,
    data: Option<PageEnvData>,
}

/// HTTP 版本：调用 REST API 获取环境列表
async fn api_list_envs(api_key: &str, page: u32, page_size: u32) -> Result<PageEnvData, String> {
    let client = reqwest::Client::new();

    let body = PageEnvRequest {
        customer_id: "default".to_string(),
        page,
        page_size,
    };

    tracing::info!("list envs request: {}", serde_json::to_string(&body).unwrap_or_default());

    let resp = client
        .post(PAGE_ENV_URL)
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

    tracing::info!("list envs response: {}", body);

    let result: PageEnvResponse = serde_json::from_str(&body)
        .map_err(|e| format!("解析响应失败: {}", e))?;

    if result.code != 200 {
        return Err(format!("获取环境列表失败: {} (code={})", result.msg, result.code));
    }

    result.data.ok_or_else(|| "响应中缺少 data 字段".to_string())
}

/// SDK 版本：调用 sdk_env_page 获取环境列表
async fn api_list_envs2(page: u32, page_size: u32) -> Result<PageEnvData, String> {
    // 调用 SDK 的 sdk_env_page 接口
    let body = format!(r#"{{"page":{},"pageSize":{}}}"#, page, page_size);
    tracing::info!("sdk_env_page request: {}", body);

    let response = manager::sdk_env_page(&body)?;

    tracing::info!("sdk_env_page response: {}", response);

    // 解析返回的 JSON
    let result: PageEnvResponse = serde_json::from_str(&response)
        .map_err(|e| format!("解析响应失败: {}", e))?;

    if result.code != 200 {
        return Err(format!("获取环境列表失败: {} (code={})", result.msg, result.code));
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
            let work_dir = std::env::temp_dir()
                .join(".brosdk")
                .to_string_lossy()
                .to_string();
            // 确保工作目录存在
            std::fs::create_dir_all(&work_dir)
                .map_err(|e| format!("创建工作目录失败: {}", e))?;
            tracing::info!("Work directory: {}", work_dir);
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

/// 创建环境 — 调用 SDK 的 sdk_env_create，返回新建的 envId
#[tauri::command]
pub async fn create_env(
    kernel_version: String,
    proxy: Option<String>,
    bridge_proxy: Option<String>,
    state: State<'_, AppState>,
) -> Result<String, String> {
    if !*state.initialized.lock().unwrap() {
        return Err("SDK 未初始化".to_string());
    }

    // 使用与 HTTP 版本相同的 CreateEnvRequest 结构体构建参数
    let request = CreateEnvRequest {
        customer_id: "default".to_string(),
        device_name: "brosdk-demo".to_string(),
        env_name: format!("env-{}", chrono::Utc::now().timestamp()),
        finger: FingerConfig {
            kernel: "Chrome".to_string(),
            kernel_version: kernel_version,
            system: "All Windows".to_string(),
            public_ip: "127.0.0.1".to_string(),
        },
        proxy: proxy,
        bridge_proxy: bridge_proxy,
    };
    let config = serde_json::to_string(&request)
        .map_err(|e| format!("序列化请求失败: {}", e))?;

    tracing::info!("sdk_env_create request: {}", config);

    let result = manager::sdk_env_create(&config)?;

    // 使用与 HTTP 版本相同的响应结构进行解析
    let resp: CreateEnvResponse = serde_json::from_str(&result)
        .map_err(|e| format!("解析创建结果失败: {}", e))?;

    if resp.code != 200 {
        return Err(format!("创建环境失败: {} (code={})", resp.msg, resp.code));
    }

    let data = resp.data.ok_or_else(|| "响应中缺少 data 字段".to_string())?;

    tracing::info!("Environment created via SDK: {} ({})", data.env_name, data.env_id);
    Ok(data.env_id)
}

/// 创建环境 — 调用 REST API（HTTP 版本），返回新建的 envId
#[tauri::command]
pub async fn create_env_http(
    kernel_version: String,
    proxy: Option<String>,
    bridge_proxy: Option<String>,
    state: State<'_, AppState>,
) -> Result<String, String> {
    if !*state.initialized.lock().unwrap() {
        return Err("SDK 未初始化".to_string());
    }

    let api_key = state.api_key.lock().unwrap().clone();

    let data = api_create_env(&api_key, &kernel_version, proxy.as_deref(), bridge_proxy.as_deref()).await?;
    tracing::info!("Environment created via HTTP: {} ({})", data.env_name, data.env_id);
    Ok(data.env_id)
}

/// 启动环境
#[tauri::command]
pub async fn start_env(env_id: String, state: State<'_, AppState>) -> Result<String, String> {
    if !*state.initialized.lock().unwrap() {
        return Err("SDK 未初始化".to_string());
    }

    let config = serde_json::json!({
        "envs": [{
            "envId": env_id,
            "args": ["--no-first-run", "--no-default-browser-check", "--remote-debugging-port=9222"],
        }]
    });
    let json = config.to_string();

    match manager::browser_open(&json) {
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

/// 获取环境列表 — 调用 sdk_env_page SDK 接口，返回环境列表供下拉选择
/// 返回 (envId, envName, kernelVersion) 三元组
#[tauri::command]
pub async fn list_envs(
    page: Option<u32>,
    page_size: Option<u32>,
    state: State<'_, AppState>,
) -> Result<Vec<(String, String, String)>, String> {
    if !*state.initialized.lock().unwrap() {
        return Err("SDK 未初始化".to_string());
    }

    // 调用 SDK 的 sdk_env_page 接口
    let data = api_list_envs2(page.unwrap_or(1), page_size.unwrap_or(50)).await?;

    tracing::info!("Fetched {} environments (total: {})", data.list.len(), data.total);

    // 转换为 (envId, envName, kernelVersion) 三元组列表
    Ok(data
        .list
        .into_iter()
        .map(|e| {
            let kernel = e.finger.kernel_version.unwrap_or_default();
            (e.env_id, e.env_name, kernel)
        })
        .collect())
}

/// 获取环境列表 — 调用 REST API（HTTP 版本），返回环境列表供下拉选择
/// 返回 (envId, envName, kernelVersion) 三元组
/// 注意：此版本只需要 API Key，不需要 SDK 初始化
#[tauri::command]
pub async fn list_envs2(
    api_key: String,
    page: Option<u32>,
    page_size: Option<u32>,
) -> Result<Vec<(String, String, String)>, String> {
    if api_key.is_empty() {
        return Err("API Key 未配置，请先填写 API Key".to_string());
    }

    let data = api_list_envs(&api_key, page.unwrap_or(1), page_size.unwrap_or(50)).await?;

    tracing::info!("Fetched {} environments via HTTP (total: {})", data.list.len(), data.total);

    // 转换为 (envId, envName, kernelVersion) 三元组列表
    Ok(data
        .list
        .into_iter()
        .map(|e| {
            let kernel = e.finger.kernel_version.unwrap_or_default();
            (e.env_id, e.env_name, kernel)
        })
        .collect())
}

/// 查询 SDK 运行时信息（版本、状态等）。
/// 启动时前端主动调用，无需先完成 init_sdk。
/// 若库尚未加载则返回 Err，前端忽略错误即可。
#[tauri::command]
pub fn get_sdk_info() -> Result<String, String> {
    brosdk_sdk::sdk_info()
}

// ────────────────────────────────────────────────────────────────────────────
// SDK 下载相关
//

#[derive(Deserialize)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
}

#[derive(Deserialize)]
struct GitHubRelease {
    tag_name: String,
    assets: Vec<GitHubAsset>,
}

/// 在目录树中递归查找指定文件名，返回第一个匹配的路径。
fn find_file_recursive(dir: &std::path::Path, filename: &str) -> Option<std::path::PathBuf> {
    let entries = std::fs::read_dir(dir).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if let Some(found) = find_file_recursive(&path, filename) {
                return Some(found);
            }
        } else if path.file_name().map(|n| n == filename).unwrap_or(false) {
            return Some(path);
        }
    }
    None
}

/// 下载 SDK 库文件（从 GitHub releases）
#[tauri::command]
pub async fn download_sdk_lib() -> Result<String, String> {
    let client = reqwest::Client::new();

    // 调用 GitHub API 获取最新 release
    let resp = client
        .get("https://api.github.com/repos/browsersdk/brosdk-sdk/releases/latest")
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", "brosdk-sdk-rust")
        .send()
        .await
        .map_err(|e| format!("请求 GitHub API 失败: {}", e))?;

    let release: GitHubRelease = resp
        .json()
        .await
        .map_err(|e| format!("解析 GitHub API 响应失败: {}", e))?;

    tracing::info!("Latest SDK release: {}", release.tag_name);

    // 从 tag_name 提取版本号（去掉开头的 v）
    let version = release.tag_name.trim_start_matches('v');

    // 根据平台选择对应的压缩包，文件名包含版本号
    #[cfg(target_os = "windows")]
    let (archive_name, lib_subdir, lib_name) = (
        format!("brosdk-{}-windows-x64.zip", version),
        "windows-x64",
        "brosdk.dll",
    );

    #[cfg(target_os = "macos")]
    let (archive_name, lib_subdir, lib_name) = (
        format!("brosdk-{}-darwin-arm64.tar.gz", version),
        "macos-arm64",
        "brosdk.dylib",
    );

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    return Err("不支持的操作系统".to_string());

    let asset = release
        .assets
        .iter()
        .find(|a| a.name == archive_name)
        .ok_or_else(|| format!("未找到平台对应的 SDK 文件: {}", archive_name))?;

    tracing::info!("Downloading SDK from: {}", asset.browser_download_url);

    // 下载压缩包
    let resp = client
        .get(&asset.browser_download_url)
        .send()
        .await
        .map_err(|e| format!("下载 SDK 失败: {}", e))?;

    let bytes = resp
        .bytes()
        .await
        .map_err(|e| format!("读取 SDK 文件失败: {}", e))?;

    // 确保 libs 目标目录存在
    let lib_dir = std::path::Path::new("libs").join(lib_subdir);
    std::fs::create_dir_all(&lib_dir)
        .map_err(|e| format!("创建目录失败: {}", e))?;

    // 解压到临时目录
    let temp_dir = std::env::temp_dir().join(format!("brosdk-sdk-{}", release.tag_name));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir)
        .map_err(|e| format!("创建临时目录失败: {}", e))?;

    // ── 解压 ────────────────────────────────────────────────────────────────

    #[cfg(target_os = "windows")]
    {
        let zip_path = temp_dir.join(&archive_name);
        std::fs::write(&zip_path, &bytes)
            .map_err(|e| format!("保存压缩包失败: {}", e))?;

        let zip_file = std::fs::File::open(&zip_path)
            .map_err(|e| format!("打开压缩包失败: {}", e))?;
        let mut archive = zip::ZipArchive::new(zip_file)
            .map_err(|e| format!("解析压缩包失败: {}", e))?;

        for i in 0..archive.len() {
            let mut file = archive.by_index(i)
                .map_err(|e| format!("读取压缩包内文件失败: {}", e))?;
            let outpath = temp_dir.join(file.mangled_name());

            if file.name().ends_with('/') {
                std::fs::create_dir_all(&outpath)
                    .map_err(|e| format!("创建目录失败: {}", e))?;
            } else {
                if let Some(p) = outpath.parent() {
                    std::fs::create_dir_all(p)
                        .map_err(|e| format!("创建目录失败: {}", e))?;
                }
                let mut outfile = std::fs::File::create(&outpath)
                    .map_err(|e| format!("创建文件失败: {}", e))?;
                std::io::copy(&mut file, &mut outfile)
                    .map_err(|e| format!("写入文件失败: {}", e))?;
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        use flate2::read::GzDecoder;
        let tar = GzDecoder::new(&bytes[..]);
        let mut archive = tar::Archive::new(tar);
        archive.unpack(&temp_dir)
            .map_err(|e| format!("解压 tar.gz 失败: {}", e))?;
    }

    // ── 拷贝库文件到目标目录 ─────────────────────────────────────────────────

    let dst_path = lib_dir.join(lib_name);

    // 先在临时目录根部查找，再递归查找子目录
    let src_lib = {
        let root_candidate = temp_dir.join(lib_name);
        if root_candidate.exists() {
            root_candidate
        } else {
            find_file_recursive(&temp_dir, lib_name)
                .ok_or_else(|| format!("解压后未找到 {} 文件", lib_name))?
        }
    };

    tracing::info!("Copying {} -> {}", src_lib.display(), dst_path.display());
    std::fs::copy(&src_lib, &dst_path)
        .map_err(|e| format!("复制库文件失败: {}", e))?;

    // 清理临时目录
    let _ = std::fs::remove_dir_all(&temp_dir);

    let file_size = std::fs::metadata(&dst_path)
        .map_err(|e| format!("获取文件大小失败: {}", e))?
        .len() / 1024;

    tracing::info!("SDK extracted and copied to: {}", dst_path.display());

    Ok(format!(
        "SDK {} 已下载并解压到 {} ({} KB)",
        release.tag_name,
        dst_path.display(),
        file_size
    ))
}
