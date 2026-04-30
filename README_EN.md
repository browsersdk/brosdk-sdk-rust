# BroSDK Rust SDK

English | [简体中文](README.md)

[![Crates.io](https://img.shields.io/crates/v/brosdk)](https://crates.io/crates/brosdk)
[![Rust](https://img.shields.io/badge/rust-2021-orange.svg)](https://www.rust-lang.org/)
![License: MIT](https://img.shields.io/badge/license-MIT-green.svg)

`brosdk-rust` is the Rust binding for BroSDK and also includes a Tauri v2 desktop demo. It dynamically loads the BroSDK native library through `libloading` and wraps environment management, browser control, SDK information, token updates, and asynchronous callbacks as Rust APIs.

It is suitable for Rust services, CLI tools, desktop clients, Tauri applications, and systems that need strongly typed integration with BroSDK browser environment capabilities.

## Core Capabilities

- Dynamically load BroSDK native libraries on Windows and macOS.
- Initialize the SDK with `userSig`, a working directory, and a local service port.
- Create, query, launch, and close browser environments.
- Bridge SDK asynchronous callbacks to the Tauri event `brosdk-event`.
- Support both pure Rust library mode and Tauri desktop application mode.
- Provide a static demo UI that can run without a separate frontend build step.

## Requirements

| Item | Requirement |
|------|-------------|
| Rust | 2021 edition |
| Tauri | v2, required only when the default `tauri-app` feature is enabled |
| Native library | `brosdk.dll` or `brosdk.dylib` |
| Authentication | BroSDK API Key or a valid `userSig` |

Native libraries can be downloaded from [brosdk releases](https://github.com/browsersdk/brosdk/releases) and placed under:

```text
libs/
├── windows-x64/brosdk.dll
└── macos-arm64/brosdk.dylib
```

## Installation

Add to `Cargo.toml`:

```toml
[dependencies]
brosdk = "1"
```

If you only need the Rust library without Tauri integration:

```toml
[dependencies]
brosdk = { version = "1", default-features = false }
```

## Quick Start

### Tauri Integration Mode

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

Listen for SDK events:

```rust
use brosdk::SdkEvent;

app.listen("brosdk-event", |event| {
    let sdk_event: SdkEvent = serde_json::from_str(event.payload()).unwrap();
    println!("code={} data={}", sdk_event.code, sdk_event.data);
});
```

Frontend listener:

```js
const { listen } = window.__TAURI__.event;

await listen("brosdk-event", ({ payload }) => {
  console.log(payload);
});
```

### Pure Rust Mode

After disabling default features, `load` does not require `AppHandle`, and SDK callbacks are printed to stdout.

```rust
brosdk::load("libs/windows-x64/brosdk.dll")?;
brosdk::init("your_user_sig", "./workDir", 8080)?;
brosdk::browser_open("env-001")?;
brosdk::shutdown()?;
```

## Run The Demo

```bash
cargo build
cargo run --bin brosdk-demo
```

Release build:

```bash
cargo build --release --bin brosdk-demo
```

The Tauri window loads `dist/index.html`, so no separate frontend build step is required.

Demo flow:

1. Enter an API Key and initialize the SDK. The application exchanges it for `userSig` through the REST API.
2. Query the environment list or create a new environment by selecting a core version.
3. Launch the environment and wait for the `brosdk-event` launch result.
4. Close the environment or inspect SDK information.

## API Overview

### Core Functions

| Function | Description |
|----------|-------------|
| `load(app, path)` | Load the native library and register result and Cookie/Storage callbacks |
| `init(user_sig, work_dir, port)` | Initialize the SDK |
| `browser_open(env_id)` | Launch a browser environment; result is returned by event callback |
| `browser_close(env_id)` | Close a browser environment |
| `token_update(token_json)` | Refresh access token |
| `sdk_info()` | Query SDK runtime information |
| `sdk_env_create(config_json)` | Create a browser environment |
| `sdk_env_page(page_json)` | Query environments by page |
| `shutdown()` | Shut down the SDK and release resources |

### Tauri Commands

| Command | Description |
|---------|-------------|
| `init_sdk` | Initialize SDK with API Key |
| `create_env` | Create a browser environment |
| `start_env` | Launch a browser environment |
| `stop_env` | Close a browser environment |
| `get_sdk_info` | Get SDK information |
| `list_envs` | Query environment list |

### Event Structure

```rust
pub struct SdkEvent {
    pub code: i32,
    pub data: String,
}
```

APIs such as `browser_open` may be asynchronous. Function return values only indicate whether the request was submitted successfully; final status should be determined from `brosdk-event`.

## Feature

| Feature | Default | Description |
|---------|---------|-------------|
| `tauri-app` | yes | Enable Tauri integration, `AppHandle`, and event emission |

## Directory Layout

```text
brosdk-rust/
├── Cargo.toml              # Rust crate configuration
├── build.rs                # Tauri build script
├── tauri.conf.json         # Tauri window and bundle configuration
├── src/                    # Rust library source
│   ├── lib.rs
│   └── brosdk/
│       ├── ffi.rs          # Raw FFI binding
│       └── manager.rs      # High-level wrapper and event bridge
├── src-tauri/              # Tauri demo entry and commands
├── dist/index.html         # Static demo UI
├── libs/                   # Native library directory
└── tests/                  # Integration tests
```

## macOS Packaging Notes

macOS applications must be packaged on a macOS host. The Tauri configuration copies `libs/**/*` into the `.app` bundle as resources, and runtime code resolves the dynamic library through the resource directory.

```bash
rustup target add aarch64-apple-darwin x86_64-apple-darwin
npm install -g @tauri-apps/cli

cargo tauri build --target aarch64-apple-darwin
```

Check Bundle ID after packaging:

```bash
/usr/libexec/PlistBuddy -c "Print CFBundleIdentifier" \
  "src-tauri/target/*-apple-darwin/release/bundle/macos/Brosdk Demo.app/Contents/Info.plist"
```

When launching browser environments on macOS, `--parent-bundle-identifier=com.brosdk.demo` is appended so the browser process inherits permissions from the host application.

## Related Repositories

| Repository | Description |
|------------|-------------|
| [brosdk](https://github.com/browsersdk/brosdk) | Native C/C++ SDK |
| [brosdk-core](https://github.com/browsersdk/brosdk-core) | Browser core versions and platform support |
| [brosdk-docs](https://github.com/browsersdk/brosdk-docs) | Official documentation and API reference |
| [browser-demo](https://github.com/browsersdk/browser-demo) | Full server-side and desktop client example |

## License

MIT
