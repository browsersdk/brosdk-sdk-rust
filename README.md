# BroSDK Rust SDK

[English](README_EN.md) | 简体中文

[![Crates.io](https://img.shields.io/crates/v/brosdk)](https://crates.io/crates/brosdk)
[![Rust](https://img.shields.io/badge/rust-2021-orange.svg)](https://www.rust-lang.org/)
![License: MIT](https://img.shields.io/badge/license-MIT-green.svg)

`brosdk-rust` 是 BroSDK 的 Rust 语言绑定，同时包含一个 Tauri v2 桌面应用 Demo。它通过 `libloading` 动态加载 BroSDK 原生库，将环境管理、浏览器控制、SDK 信息查询、Token 更新和异步事件回调封装为 Rust API。

适用于 Rust 服务、CLI 工具、桌面客户端、Tauri 应用，以及需要以强类型方式集成 BroSDK 浏览器环境能力的系统。

## 核心能力

- 动态加载 Windows / macOS 平台的 BroSDK 原生库。
- 使用 `userSig` 初始化 SDK，并指定工作目录和本地服务端口。
- 创建、查询、启动和关闭浏览器环境。
- 将 SDK 异步回调桥接为 Tauri 事件 `brosdk-event`。
- 提供纯 Rust 库模式和 Tauri 桌面应用模式。
- 内置静态 Demo UI，无需额外前端构建即可运行示例。

## 环境要求

| 项目 | 要求 |
|------|------|
| Rust | 2021 edition |
| Tauri | v2，仅在默认 `tauri-app` feature 下需要 |
| 原生库 | `brosdk.dll` 或 `brosdk.dylib` |
| 认证 | BroSDK API Key 或可用的 `userSig` |

原生库可从 [brosdk releases](https://github.com/browsersdk/brosdk/releases) 获取，并放置到：

```text
libs/
├── windows-x64/brosdk.dll
└── macos-arm64/brosdk.dylib
```

## 安装

添加到 `Cargo.toml`：

```toml
[dependencies]
brosdk = "1"
```

如果只使用 Rust 库能力、不需要 Tauri 集成：

```toml
[dependencies]
brosdk = { version = "1", default-features = false }
```

## 快速开始

### Tauri 集成模式

```rust
use brosdk::{browser_close, browser_open, init, load, shutdown};

fn setup(app_handle: tauri::AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    load(app_handle, "libs/windows-x64/brosdk.dll")?;

    init("your_user_sig", "./workDir", 8080)?;

    browser_open("env-001")?;
    browser_close("env-001")?;

    shutdown()?;
    Ok(())
}
```

监听 SDK 事件：

```rust
use brosdk::SdkEvent;

app.listen("brosdk-event", |event| {
    let sdk_event: SdkEvent = serde_json::from_str(event.payload()).unwrap();
    println!("code={} data={}", sdk_event.code, sdk_event.data);
});
```

前端监听：

```js
const { listen } = window.__TAURI__.event;

await listen("brosdk-event", ({ payload }) => {
  console.log(payload);
});
```

### 纯 Rust 模式

禁用默认 feature 后，`load` 不需要 `AppHandle`，SDK 回调会输出到标准输出。

```rust
brosdk::load("libs/windows-x64/brosdk.dll")?;
brosdk::init("your_user_sig", "./workDir", 8080)?;
brosdk::browser_open("env-001")?;
brosdk::shutdown()?;
```

## 运行 Demo

```bash
cargo build
cargo run --bin brosdk-demo
```

Release 构建：

```bash
cargo build --release --bin brosdk-demo
```

Tauri 窗口加载 `dist/index.html`，无需单独执行前端构建。

Demo 流程：

1. 输入 API Key 并初始化 SDK，程序会通过 REST API 换取 `userSig`。
2. 查询环境列表，或选择内核版本创建新环境。
3. 启动环境并等待 `brosdk-event` 返回启动结果。
4. 关闭环境或查看 SDK 信息。

## API 概览

### 核心函数

| 函数 | 说明 |
|------|------|
| `load(app, path)` | 加载原生库并注册结果、Cookie/Storage 回调 |
| `init(user_sig, work_dir, port)` | 初始化 SDK |
| `browser_open(env_id)` | 启动浏览器环境，结果通过事件返回 |
| `browser_close(env_id)` | 关闭浏览器环境 |
| `token_update(token_json)` | 刷新访问令牌 |
| `sdk_info()` | 查询 SDK 运行时信息 |
| `sdk_env_create(config_json)` | 创建浏览器环境 |
| `sdk_env_page(page_json)` | 分页查询环境列表 |
| `shutdown()` | 关闭 SDK 并释放资源 |

### Tauri 命令

| 命令 | 说明 |
|------|------|
| `init_sdk` | 使用 API Key 初始化 SDK |
| `create_env` | 创建浏览器环境 |
| `start_env` | 启动浏览器环境 |
| `stop_env` | 关闭浏览器环境 |
| `get_sdk_info` | 获取 SDK 信息 |
| `list_envs` | 查询环境列表 |

### 事件结构

```rust
pub struct SdkEvent {
    pub code: i32,
    pub data: String,
}
```

`browser_open` 等接口可能是异步操作，函数返回值只表示请求是否提交成功，最终状态应通过 `brosdk-event` 判断。

## Feature

| Feature | 默认 | 说明 |
|---------|------|------|
| `tauri-app` | yes | 启用 Tauri 集成、`AppHandle` 和事件发射 |

## 目录结构

```text
brosdk-rust/
├── Cargo.toml              # Rust crate 配置
├── build.rs                # Tauri 构建脚本
├── tauri.conf.json         # Tauri 窗口和打包配置
├── src/                    # Rust 库源码
│   ├── lib.rs
│   └── brosdk/
│       ├── ffi.rs          # 原始 FFI 绑定
│       └── manager.rs      # 高级封装和事件桥接
├── src-tauri/              # Tauri Demo 入口和命令
├── dist/index.html         # 静态 Demo UI
├── libs/                   # 原生动态库放置目录
└── tests/                  # 集成测试
```

## macOS 打包说明

macOS 应用必须在 macOS 主机上打包。Tauri 配置会将 `libs/**/*` 作为资源复制进 `.app`，运行时通过资源目录定位动态库。

```bash
rustup target add aarch64-apple-darwin x86_64-apple-darwin
npm install -g @tauri-apps/cli

cargo tauri build --target aarch64-apple-darwin
```

打包后可检查 Bundle ID：

```bash
/usr/libexec/PlistBuddy -c "Print CFBundleIdentifier" \
  "src-tauri/target/*-apple-darwin/release/bundle/macos/Brosdk Demo.app/Contents/Info.plist"
```

macOS 启动浏览器环境时会追加 `--parent-bundle-identifier=com.brosdk.demo`，使浏览器进程继承宿主应用权限。

## 与 BroSDK 生态的关系

| 仓库 | 说明 |
|------|------|
| [brosdk](https://github.com/browsersdk/brosdk) | 原生 C/C++ SDK |
| [brosdk-core](https://github.com/browsersdk/brosdk-core) | 浏览器内核版本和平台支持 |
| [brosdk-docs](https://github.com/browsersdk/brosdk-docs) | 官方文档和 API 参考 |
| [browser-demo](https://github.com/browsersdk/browser-demo) | 完整服务端和桌面客户端示例 |

## License

MIT
