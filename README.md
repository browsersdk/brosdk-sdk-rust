# brosdk-rust

Rust 语言绑定库 + Tauri 桌面应用 Demo。

动态加载平台 DLL/dylib 并暴露安全、符合 Rust 惯用法的 API，支持可选的 Tauri 集成。

## 项目结构

```
brosdk-rust/
├── Cargo.toml              # 库配置（lib + demo binary）
├── build.rs                # Tauri 构建脚本
├── tauri.conf.json         # Tauri 窗口/打包配置
├── libs/
│   ├── windows-x64/        # brosdk.dll（需手动放置）
│   └── macos-arm64/        # brosdk.dylib
├── src/                    # 库 crate
│   ├── lib.rs              # 公共 API 导出
│   └── brosdk/
│       ├── ffi.rs          # 原始 C FFI 绑定（libloading）
│       └── manager.rs       # 高级安全封装 + Tauri 事件桥接
├── src-tauri/              # Tauri demo 应用
│   ├── main.rs             # 应用入口
│   └── commands.rs         # Tauri invoke 命令
├── dist/
│   └── index.html          # Demo UI（纯静态，无需构建）
├── gen/                    # Tauri 生成的代码
├── icons/                  # 应用图标
└── package.json            # npm 配置（Tauri 依赖）
```

## 功能特性

| 特性 | 默认 | 说明 |
|------|------|------|
| `tauri-app` | yes | 启用 Tauri 集成（`AppHandle`，事件发射） |

不启用此特性时，`load(lib_path)` 不需要 `AppHandle`，SDK 回调仅输出到 stdout。

## 环境要求

- Rust 2021 edition
- Tauri v2 前置条件 — 参见 [tauri.app/start/prerequisites](https://v2.tauri.app/start/prerequisites/)
- 从 [github.com/browsersdk/brosdk/releases](https://github.com/browsersdk/brosdk/releases) 下载原生库并放置到 `libs/` 目录：

```
libs/
├── windows-x64/brosdk.dll
└── macos-arm64/brosdk.dylib
```

### macOS 打包注意事项

1. **动态库打包**：Tauri 配置了 `bundle.resources`，打包时会将 `libs/**/*` 复制到 `.app/Contents/Frameworks/` 目录

2. **Bundle 内的库路径**：代码中使用 `app.path().resource_dir()` 获取资源目录，拼接为完整路径：
   - macOS：`{resource_dir}/Frameworks/brosdk.dylib`
   - Windows：`{resource_dir}/Frameworks/brosdk.dll`

3. **开发模式 vs 生产模式**：
   - 开发 `cargo run`：使用项目根目录的相对路径 `libs/macos-arm64/brosdk.dylib`
   - 打包后：使用 `app.path().resource_dir()` 指向的 bundle 资源目录

## 运行 Demo

```bash
# Debug 构建
cargo build

# 运行 Demo
cargo run --bin brosdk-demo

# Release 构建
cargo build --release
```

Tauri 窗口加载 `dist/index.html`，无需前端构建步骤。

### Demo 使用流程

1. **填写 API Key** → 点击 **初始化 SDK**：通过 REST API 用 API Key 换取 `userSig`，然后初始化原生 SDK
2. **点击环境列表**（或手动输入 envId）：选择或填写环境 ID
3. 选择内核版本 → 点击 **创建环境**：调用 REST API 创建新的浏览器环境
4. 点击 **启动环境** / **关闭环境** 控制浏览器环境

### UI 功能说明

- **环境列表弹框**：点击"环境列表"按钮弹出环境选择框，支持搜索和选择已有环境
- **自动填充**：选择环境后自动填充对应的内核版本
- **输入持久化**：API Key、envId 等输入自动保存到 localStorage

## 库使用方式

添加到 `Cargo.toml`：

```toml
[dependencies]
brosdk = "1.0.1"
```

或使用最新版本：

```toml
[dependencies]
brosdk = "1"
```

### 与 Tauri 集成

```rust
use brosdk::{load, init, browser_open, browser_close, shutdown};

// 加载原生库并注册回调
load(app_handle, "libs/windows-x64/brosdk.dll")?;

// 用 userSig 初始化（通过交换 API Key 获取）
init("your_user_sig", "/path/to/work_dir", 8080)?;

// 打开浏览器环境 — 结果通过 "brosdk-event" 事件返回
browser_open("env-001")?;

// 关闭浏览器环境
browser_close("env-001")?;

shutdown()?;
```

### 不使用 Tauri（特性 = 无 `tauri-app`）

```rust
brosdk::load("libs/windows-x64/brosdk.dll")?;
brosdk::init("your_user_sig", "/path/to/work_dir", 8080)?;
```

### 监听 SDK 事件（Tauri）

库会为每个异步 SDK 回调发射 `brosdk-event` Tauri 事件。

```rust
use brosdk::SdkEvent;

app.listen("brosdk-event", |event| {
    let e: SdkEvent = serde_json::from_str(event.payload()).unwrap();
    println!("code={} data={}", e.code, e.data);
});
```

前端：

```js
const { listen } = window.__TAURI__.event;
await listen("brosdk-event", ({ payload }) => console.log(payload));
```

## API 参考

### 核心函数

| 函数 | 说明 |
|------|------|
| `load(app, path)` | 加载原生库，注册结果和 cookies-storage 回调 |
| `init(user_sig, work_dir, port)` | 用凭据初始化 SDK，返回 JSON 结果字符串 |
| `browser_open(env_id)` | 启动浏览器环境（异步 — 结果通过 `brosdk-event`） |
| `browser_close(env_id)` | 关闭浏览器环境 |
| `token_update(token_json)` | 刷新访问令牌 |
| `shutdown()` | 优雅关闭 |
| `sdk_info()` | 查询 SDK 运行时信息（版本、状态等） |
| `sdk_env_create(config_json)` | 创建新环境，返回环境信息 JSON |
| `sdk_env_page(page_json)` | 分页查询环境列表 |

### Tauri 命令

| 命令 | 说明 |
|------|------|
| `init_sdk` | 初始化 SDK（需要 apiKey） |
| `create_env` | 创建环境（需要 kernelVersion） |
| `start_env` | 启动环境（需要 envId） |
| `stop_env` | 关闭环境（需要 envId） |
| `get_sdk_info` | 获取 SDK 信息 |
| `list_envs` | 获取环境列表（SDK 方式，需要 SDK 初始化） |

### `SdkEvent`

```rust
pub struct SdkEvent {
    pub code: i32,    // SDK 状态码
    pub data: String, // 原生回调的 JSON 数据
}
```

## 构建

### 项目结构说明

本项目是 **Rust 库 + Tauri 桌面应用** 的组合，`tauri.conf.json` 位于根目录（而非标准 `src-tauri/` 下）。

```
brosdk-sdk-rust/
├── Cargo.toml            ← lib + binary 定义
├── tauri.conf.json       ← Tauri 配置（根目录）
├── build.rs              ← Tauri 构建脚本
├── src/lib.rs            ← brosdk 库（FFI 绑定）
├── src-tauri/main.rs     ← Tauri 应用入口
└── dist/index.html       ← 纯静态前端（无需构建）
```

### 通用构建

```bash
# Debug 构建
cargo build --bin brosdk-demo

# Release 构建
cargo build --release --bin brosdk-demo

# 仅编译 Rust 库（不含 Tauri UI）
cargo build --release --lib --no-default-features
```

产物：`target/release/brosdk-demo.exe`（Windows）或 `target/release/brosdk-demo`（macOS）

### macOS 打包

> ⚠️ **macOS 打包必须在 macOS 主机上执行**，无法在 Windows/Linux 交叉编译。

#### 前置要求

```bash
# 安装 Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 添加 macOS 编译目标
rustup target add aarch64-apple-darwin x86_64-apple-darwin

# 安装 Xcode Command Line Tools
xcode-select --install

# 安装 Tauri CLI
npm install -g @tauri-apps/cli
```

#### 打包命令

```bash
# 打包 macOS 应用（.app + .dmg）
cargo tauri build --target aarch64-apple-darwin
# 或 x86_64
cargo tauri build --target x86_64-apple-darwin
```

#### 编译产物

| 产物 | 路径 |
|------|------|
| .app 应用 | `src-tauri/target/*-apple-darwin/release/bundle/macos/Brosdk Demo.app` |
| .dmg 安装包 | `src-tauri/target/*-apple-darwin/release/bundle/macos/Brosdk Demo.dmg` |

#### Bundle ID 说明

- **配置位置**：`tauri.conf.json` → `identifier: "com.brosdk.demo"`
- **生效位置**：`Info.plist` → `CFBundleIdentifier`

验证命令：

```bash
# 编译后读取 Info.plist
/usr/libexec/PlistBuddy -c "Print CFBundleIdentifier" \
  "src-tauri/target/*-apple-darwin/release/bundle/macos/Brosdk Demo.app/Contents/Info.plist"

# 或安装后查询
mdls -name kMDItemCFBundleIdentifier "/Applications/Brosdk Demo.app"
```

#### macOS 启动参数

启动浏览器环境时，macOS 下会自动追加 `--parent-bundle-identifier=com.brosdk.demo` 参数，让浏览器进程继承宿主 App 的权限（沙盒、Keychain 等）。

## 协议

MIT
