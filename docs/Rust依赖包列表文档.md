# Rust开发所需包列表文档

AI-Commit TUI项目完整依赖包清单

## 1. 核心依赖包

### 1.1 TUI界面框架

```toml
[dependencies]
# 终端用户界面框架
ratatui = "0.24"

# 跨平台终端处理
crossterm = "0.27"

# 终端颜色支持
owo-colors = "3.5"
```

### 1.2 Git操作

```toml
[dependencies]
# Git操作的Rust绑定 (基于libgit2)
git2 = "0.18"

# Git配置解析
git-config = "0.2"

# Git对象处理
gix = "0.56"  # 备选纯Rust Git实现
```

### 1.3 异步运行时

```toml
[dependencies]
# 异步运行时
tokio = { version = "1.35", features = [
    "full",
    "macros",
    "rt-multi-thread",
    "time",
    "io-util",
    "fs",
    "process",
    "signal"
] }

# 异步任务处理
tokio-util = { version = "0.7", features = ["full"] }

# 异步stream处理
futures = "0.3"
futures-util = "0.3"
```

## 2. 网络和HTTP客户端

### 2.1 HTTP客户端 (AI服务调用)

```toml
[dependencies]
# HTTP客户端
reqwest = { version = "0.11", features = [
    "json",
    "rustls-tls",
    "stream",
    "cookies",
    "gzip"
] }

# URL处理
url = "2.5"

# HTTP头部处理
http = "0.2"
```

### 2.2 WebSocket支持 (MCP传输层)

```toml
[dependencies]
# WebSocket客户端和服务器 (用于MCP WebSocket传输)
tokio-tungstenite = { version = "0.20", features = ["rustls-tls-webpki-roots"] }

# WebSocket服务器框架
warp = { version = "0.3", features = ["tls"] }

# 异步WebSocket处理
futures-util = { version = "0.3", features = ["sink", "async-await-macro"] }
```

### 2.3 MCP协议支持

```toml
[dependencies]
# JSON-RPC 2.0协议 (MCP基于此协议)
jsonrpc-core = "18.0"
jsonrpc-derive = "18.0"

# 协议缓冲区 (用于MCP二进制传输)
prost = "0.12"
prost-types = "0.12"

# 消息pack序列化 (可选的MCP传输格式)
rmp-serde = "1.1"
```

## 3. 序列化和配置

### 3.1 序列化框架

```toml
[dependencies]
# JSON序列化
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# TOML配置文件
toml = "0.8"
serde_toml = "0.8"

# YAML支持 (可选)
serde_yaml = "0.9"
```

### 3.2 配置管理

```toml
[dependencies]
# 配置管理框架
config = "0.13"

# 环境变量处理
dotenv = "0.15"
```

## 4. 命令行处理

### 4.1 参数解析

```toml
[dependencies]
# 命令行参数解析
clap = { version = "4.4", features = [
    "derive",
    "env",
    "unicode",
    "wrap_help",
    "color"
] }

# 命令行补全
clap_complete = "4.4"
```

## 5. 日志和调试

### 5.1 日志系统

```toml
[dependencies]
# 结构化日志框架
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = [
    "env-filter",
    "json",
    "chrono",
    "ansi"
] }

# 日志appender
tracing-appender = "0.2"

# 性能追踪
tracing-chrome = "0.7"  # Chrome DevTools格式
```

### 5.2 错误处理

```toml
[dependencies]
# 错误处理框架
thiserror = "1.0"
anyhow = "1.0"

# 错误链追踪
eyre = "0.6"
color-eyre = "0.6"  # 彩色错误输出
```

## 6. Agent架构和并发

### 6.1 Agent系统依赖

```toml
[dependencies]
# 消息传递和通道
tokio = { version = "1.35", features = ["sync", "rt-multi-thread"] }
tokio-util = { version = "0.7", features = ["sync"] }

# Actor模型框架 (可选，用于Agent实现)
actix = "0.13"

# 状态机 (用于Agent状态管理)
state_machine_future = "0.2"

# 工作窃取队列 (用于Agent任务调度)
crossbeam = "0.8"
crossbeam-queue = "0.3"

# 原子操作和并发原语
parking_lot = "0.12"
arc-swap = "1.6"
```

### 6.2 Agent通信和监控

```toml
[dependencies]
# 分布式系统工具
uuid = { version = "1.6", features = ["v4", "fast-rng"] }

# 性能监控
metrics = "0.21"
metrics-exporter-prometheus = "0.12"

# 健康检查
health-check = "0.1"

# 任务调度
tokio-cron-scheduler = "0.9"
```

## 7. 文件系统和IO

### 7.1 文件系统操作

```toml
[dependencies]
# 文件系统监控
notify = "6.1"

# 路径处理
camino = "1.1"  # UTF-8 paths

# 临时文件
tempfile = "3.8"

# 文件锁
file-lock = "2.1"
```

### 7.2 压缩和归档

```toml
[dependencies]
# 压缩支持
flate2 = "1.0"
tar = "0.4"
zip = "0.6"
```

## 8. 数据结构和算法

### 8.1 集合和数据结构

```toml
[dependencies]
# 索引映射
indexmap = "2.1"

# LRU缓存
lru = "0.12"

# 有序映射
im = "15.1"

# 位操作
bitvec = "1.0"
```

### 8.2 字符串处理

```toml
[dependencies]
# 字符串相似度
strsim = "0.10"

# 正则表达式
regex = "1.10"

# Unicode处理
unicode-width = "0.1"
unicode-segmentation = "1.10"

# 模糊搜索
fuzzy-matcher = "0.3"
```

## 9. 语法高亮

### 9.1 语法高亮引擎

```toml
[dependencies]
# 语法高亮
syntect = "5.1"

# Tree-sitter语法解析 (备选)
tree-sitter = "0.20"
tree-sitter-highlight = "0.20"

# 语言检测
tree-sitter-rust = "0.20"
tree-sitter-javascript = "0.20"
tree-sitter-python = "0.20"
tree-sitter-json = "0.20"
tree-sitter-yaml = "0.20"
tree-sitter-toml = "0.20"
```

## 10. 加密和安全

### 10.1 加密库

```toml
[dependencies]
# 加密框架
ring = "0.16"

# 密码哈希
argon2 = "0.5"

# 随机数生成
rand = "0.8"

# 基础64编码
base64 = "0.21"
```

### 10.2 TLS支持

```toml
[dependencies]
# TLS实现
rustls = "0.21"
rustls-webpki = "0.102"
webpki-roots = "0.25"
```

## 11. 测试依赖

### 11.1 测试框架

```toml
[dev-dependencies]
# 测试工具
tokio-test = "0.4"
test-log = "0.2"

# 断言增强
assert_matches = "1.5"
predicates = "3.0"

# 快照测试
insta = "1.34"

# 属性测试
proptest = "1.4"
quickcheck = "1.0"

# Agent和MCP测试工具
mockall = "0.11"  # Mock框架，用于Agent测试
wiremock = "0.5"  # HTTP mock，用于MCP测试
```

### 11.2 模拟和mock

```toml
[dev-dependencies]
# HTTP模拟
mockito = "1.2"
wiremock = "0.5"

# WebSocket模拟 (用于MCP WebSocket测试)
tungstenite = "0.20"

# Agent行为模拟
fake = "2.9"

# 时间模拟
mock_instant = "0.3"

# 文件系统模拟
tempfile = "3.8"
```

## 12. 性能和分析

### 12.1 性能分析

```toml
[dev-dependencies]
# 基准测试
criterion = { version = "0.5", features = ["html_reports"] }

# 内存分析
dhat = "0.3"

# CPU分析
pprof = { version = "0.13", features = ["flamegraph", "protobuf-codec"] }

# Agent性能分析
tokio-metrics = "0.3"
console-subscriber = "0.2"  # Tokio console支持
```

### 12.2 内存管理

```toml
[dependencies]
# 内存池
bumpalo = "3.14"

# 字符串处理优化
compact_str = "0.7"

# 小向量优化
smallvec = "1.11"

# 堆分配优化
mimalloc = { version = "0.1", optional = true }

# Agent内存管理
dashmap = "5.5"  # 并发HashMap，用于Agent状态管理
```

## 13. 平台特定依赖

### 13.1 Windows平台

```toml
[target.'cfg(windows)'.dependencies]
# Windows API
winapi = { version = "0.3", features = [
    "winuser",
    "consoleapi",
    "processenv",
    "wincon"
] }

# Windows注册表
winreg = "0.52"
```

### 13.2 Unix平台

```toml
[target.'cfg(unix)'.dependencies]
# Unix信号处理
signal-hook = "0.3"
signal-hook-tokio = { version = "0.3", features = ["futures-v0_3"] }

# 终端控制
termios = "0.3"
```

### 13.3 macOS平台

```toml
[target.'cfg(target_os = "macos")'.dependencies]
# macOS系统集成
cocoa = "0.24"
objc = "0.2"
```

## 14. 构建工具依赖

### 14.1 构建脚本

```toml
[build-dependencies]
# 版本信息
vergen = { version = "8.2", features = [
    "build",
    "git",
    "gitcl",
    "cargo"
] }

# 协议缓冲区 (MCP协议构建)
prost-build = "0.12"
tonic-build = "0.10"  # gRPC代码生成 (如果需要gRPC传输)
```

## 15. 可选功能依赖

### 15.1 功能标志

```toml
[features]
default = ["tls", "syntax-highlighting", "ai-integration", "agent-system", "mcp-protocol"]

# TLS支持
tls = ["reqwest/rustls-tls", "rustls", "webpki-roots"]

# 语法高亮
syntax-highlighting = ["syntect", "tree-sitter"]

# AI集成
ai-integration = ["reqwest", "serde_json"]

# Agent系统
agent-system = ["tokio/sync", "crossbeam", "uuid", "metrics"]

# MCP协议支持
mcp-protocol = ["tokio-tungstenite", "jsonrpc-core", "prost", "warp"]

# 高性能内存分配器
fast-alloc = ["mimalloc"]

# 插件系统
plugins = ["libloading", "dlopen"]

# 调试功能
debug = ["tracing-chrome", "dhat", "console-subscriber"]

# 实验性功能
experimental = ["gix", "actix"]
```

### 15.2 插件系统 (可选)

```toml
[dependencies]
# 动态库加载
libloading = { version = "0.8", optional = true }
dlopen = { version = "0.1", optional = true }

# 插件接口
abi_stable = { version = "0.11", optional = true }

# WebAssembly支持 (用于安全插件)
wasmtime = { version = "13.0", optional = true }
```

## 16. 完整的Cargo.toml示例

```toml
[package]
name = "ai-commit"
version = "0.1.0"
edition = "2021"
authors = ["Your Name <your.email@example.com>"]
description = "AI-powered Git commit tool with TUI interface"
license = "MIT OR Apache-2.0"
repository = "https://github.com/yourusername/ai-commit"
keywords = ["git", "ai", "tui", "commit", "terminal"]
categories = ["command-line-utilities", "development-tools"]

[dependencies]
# 核心TUI
ratatui = "0.24"
crossterm = "0.27"
owo-colors = "3.5"

# Git操作
git2 = "0.18"

# 异步运行时
tokio = { version = "1.35", features = ["full"] }
futures = "0.3"

# HTTP客户端 (包含WebSocket支持)
reqwest = { version = "0.11", features = ["json", "rustls-tls"] }
tokio-tungstenite = { version = "0.20", features = ["rustls-tls-webpki-roots"] }

# Agent系统
uuid = { version = "1.6", features = ["v4", "fast-rng"] }
crossbeam = "0.8"
metrics = "0.21"
dashmap = "5.5"

# MCP协议
jsonrpc-core = "18.0"
prost = "0.12"
warp = { version = "0.3", features = ["tls"] }

# 序列化
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
toml = "0.8"

# 配置和CLI
clap = { version = "4.4", features = ["derive", "env"] }
config = "0.13"

# 日志和错误处理
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }
thiserror = "1.0"
anyhow = "1.0"

# 文件系统
notify = "6.1"
tempfile = "3.8"

# 数据结构
indexmap = "2.1"
lru = "0.12"

# 字符串处理
regex = "1.10"
unicode-width = "0.1"

# 语法高亮
syntect = { version = "5.1", optional = true }

[dev-dependencies]
tokio-test = "0.4"
criterion = "0.5"
insta = "1.34"
mockito = "1.2"

[features]
default = ["syntax-highlighting", "agent-system", "mcp-protocol"]
syntax-highlighting = ["syntect"]
agent-system = ["uuid", "crossbeam", "metrics", "dashmap"]
mcp-protocol = ["jsonrpc-core", "prost", "tokio-tungstenite", "warp"]
fast-alloc = ["mimalloc"]

[profile.release]
lto = "thin"
codegen-units = 1
panic = "abort"
strip = true

[profile.dev]
debug = true
opt-level = 0
```

## 17. 依赖版本管理建议

### 17.1 版本固定策略

- **主要依赖**: 使用兼容版本号 (如 "1.0")
- **API依赖**: 固定小版本号 (如 "1.2.3")
- **Agent系统依赖**: 使用稳定版本，避免频繁更新
- **MCP协议依赖**: 关注协议版本兼容性
- **开发依赖**: 可以使用更宽松的版本范围

### 17.2 定期更新

- 每月检查依赖更新
- 关注安全公告
- 测试Agent和MCP兼容性
- 更新文档

## 18. Agent和MCP特别说明

### 18.1 Agent系统依赖说明

Agent系统需要特别注意以下依赖项的版本兼容性：
- `tokio`: Agent生命周期管理的核心
- `crossbeam`: Agent间消息传递的基础
- `uuid`: Agent标识的唯一性保证
- `metrics`: Agent性能监控的关键

### 18.2 MCP协议依赖说明

MCP协议实现需要确保以下依赖的稳定性：
- `jsonrpc-core`: MCP协议的基础传输格式
- `tokio-tungstenite`: WebSocket传输层的可靠性
- `prost`: 二进制协议的序列化效率
- `warp`: HTTP传输的高性能处理

这个包列表涵盖了AI-Commit TUI项目的所有必需和可选依赖，特别加强了Agent架构和MCP协议支持，确保项目能够高效开发并具备所有所需功能。
