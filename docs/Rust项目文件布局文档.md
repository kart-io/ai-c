# Rust项目文件布局文档

AI-Commit TUI项目完整的文件结构设计

## 1. 项目根目录结构

```
ai-commit/
├── Cargo.toml                 # 项目配置和依赖
├── Cargo.lock                 # 依赖版本锁定文件
├── README.md                  # 项目说明文档
├── LICENSE                    # 许可证文件
├── CHANGELOG.md               # 版本更新日志
├── .gitignore                 # Git忽略文件
├── .env.example               # 环境变量示例
├── rust-toolchain.toml        # Rust工具链配置
├── rustfmt.toml              # 代码格式化配置
├── clippy.toml               # Clippy代码检查配置
├── deny.toml                 # 依赖审核配置
├── build.rs                  # 构建脚本
├── src/                      # 源代码目录
├── tests/                    # 集成测试
├── benches/                  # 性能测试
├── examples/                 # 示例代码
├── docs/                     # 项目文档
├── assets/                   # 静态资源
├── config/                   # 配置文件模板
├── scripts/                  # 构建和部署脚本
└── target/                   # 编译输出 (Git忽略)
```

## 2. 源代码目录结构 (src/)

```
src/
├── main.rs                   # 程序入口点
├── lib.rs                    # 库文件入口 (可选)
├── cli.rs                    # 命令行接口定义
├── config.rs                 # 全局配置管理
├── error.rs                  # 错误类型定义
├── app/                      # 应用程序核心
│   ├── mod.rs                # 模块声明
│   ├── state.rs              # 应用状态管理
│   ├── events.rs             # 事件处理系统
│   ├── actions.rs            # 动作定义
│   ├── handler.rs            # 事件处理器
│   └── lifecycle.rs          # 应用生命周期
├── ui/                       # 用户界面模块
│   ├── mod.rs                # 模块声明
│   ├── app.rs                # 主应用UI
│   ├── layout.rs             # 布局管理
│   ├── themes.rs             # 主题系统
│   ├── styles.rs             # 样式定义
│   ├── keybindings.rs        # 键盘绑定
│   ├── components/           # UI组件
│   │   ├── mod.rs
│   │   ├── header.rs         # 顶部导航栏
│   │   ├── sidebar.rs        # 侧边栏
│   │   ├── main_content.rs   # 主内容区
│   │   ├── status_bar.rs     # 状态栏
│   │   ├── modal.rs          # 模态框
│   │   ├── file_list.rs      # 文件列表组件
│   │   ├── diff_viewer.rs    # 差异查看器
│   │   ├── commit_list.rs    # 提交列表
│   │   ├── search_bar.rs     # 搜索栏
│   │   ├── ai_suggestions.rs # AI建议组件
│   │   └── progress_bar.rs   # 进度条
│   ├── widgets/              # 自定义小部件
│   │   ├── mod.rs
│   │   ├── table.rs          # 增强表格
│   │   ├── tree.rs           # 树形结构
│   │   ├── text_editor.rs    # 文本编辑器
│   │   ├── color_picker.rs   # 颜色选择器
│   │   └── syntax_highlighter.rs # 语法高亮器
│   └── utils/                # UI工具函数
│       ├── mod.rs
│       ├── colors.rs         # 颜色处理
│       ├── formatting.rs     # 格式化工具
│       ├── measurements.rs   # 尺寸计算
│       └── animations.rs     # 动画效果
├── git/                      # Git操作模块
│   ├── mod.rs                # 模块声明
│   ├── repository.rs         # 仓库操作
│   ├── status.rs             # 状态检查
│   ├── diff.rs               # 差异处理
│   ├── commit.rs             # 提交操作
│   ├── branch.rs             # 分支操作
│   ├── remote.rs             # 远程操作
│   ├── log.rs                # 日志查看
│   ├── stash.rs              # 贮藏功能
│   ├── config.rs             # Git配置
│   └── utils.rs              # Git工具函数
├── ai/                       # AI服务模块 (Agent架构)
│   ├── mod.rs                # 模块声明
│   ├── core/                 # AI核心组件
│   │   ├── mod.rs            # 模块声明
│   │   ├── agent_manager.rs  # Agent管理器
│   │   ├── message_bus.rs    # Agent间消息总线
│   │   ├── context.rs        # 上下文管理
│   │   └── session.rs        # AI会话管理
│   ├── agents/               # 具体AI代理
│   │   ├── mod.rs            # 模块声明
│   │   ├── base_agent.rs     # Agent基类/trait
│   │   ├── commit_agent.rs   # 提交消息生成Agent
│   │   ├── analysis_agent.rs # 代码分析Agent
│   │   ├── review_agent.rs   # 代码审查Agent
│   │   ├── suggest_agent.rs  # 建议生成Agent
│   │   └── search_agent.rs   # 智能搜索Agent
│   ├── providers/            # AI服务提供商
│   │   ├── mod.rs
│   │   ├── openai.rs         # OpenAI API
│   │   ├── claude.rs         # Anthropic Claude API
│   │   ├── gemini.rs         # Google Gemini API
│   │   └── local.rs          # 本地AI模型
│   ├── mcp/                  # Model Context Protocol支持
│   │   ├── mod.rs            # 模块声明
│   │   ├── client.rs         # MCP客户端
│   │   ├── server.rs         # MCP服务器
│   │   ├── protocol.rs       # MCP协议实现
│   │   ├── types.rs          # MCP类型定义
│   │   ├── handlers/         # MCP消息处理器
│   │   │   ├── mod.rs
│   │   │   ├── resource.rs   # 资源处理器
│   │   │   ├── tool.rs       # 工具处理器
│   │   │   ├── prompt.rs     # 提示处理器
│   │   │   └── completion.rs # 补全处理器
│   │   └── transport/        # 传输层
│   │       ├── mod.rs
│   │       ├── websocket.rs  # WebSocket传输
│   │       ├── http.rs       # HTTP传输
│   │       └── stdio.rs      # 标准IO传输
│   ├── tools/                # AI工具集
│   │   ├── mod.rs            # 模块声明
│   │   ├── git_tools.rs      # Git相关工具
│   │   ├── file_tools.rs     # 文件操作工具
│   │   ├── search_tools.rs   # 搜索工具
│   │   └── system_tools.rs   # 系统工具
│   ├── prompt/               # 提示词管理
│   │   ├── mod.rs            # 模块声明
│   │   ├── templates.rs      # 模板管理
│   │   ├── context.rs        # 上下文构建
│   │   ├── variables.rs      # 变量替换
│   │   └── optimization.rs   # 提示优化
│   ├── cache.rs              # AI响应缓存
│   └── config.rs             # AI配置
├── search/                   # 搜索引擎模块
│   ├── mod.rs                # 模块声明
│   ├── engine.rs             # 搜索引擎
│   ├── indexer.rs            # 索引管理
│   ├── query_parser.rs       # 查询解析
│   ├── filters.rs            # 过滤器
│   ├── ranking.rs            # 结果排序
│   ├── highlighter.rs        # 结果高亮
│   └── cache.rs              # 搜索缓存
├── storage/                  # 数据存储模块
│   ├── mod.rs                # 模块声明
│   ├── config.rs             # 配置存储
│   ├── cache.rs              # 缓存管理
│   ├── history.rs            # 历史记录
│   ├── preferences.rs        # 用户偏好
│   └── database.rs           # 本地数据库 (可选)
├── network/                  # 网络通信模块
│   ├── mod.rs                # 模块声明
│   ├── client.rs             # HTTP客户端
│   ├── retry.rs              # 重试机制
│   ├── rate_limit.rs         # 频率限制
│   └── proxy.rs              # 代理支持
├── security/                 # 安全模块
│   ├── mod.rs                # 模块声明
│   ├── encryption.rs         # 加密功能
│   ├── keychain.rs           # 密钥管理
│   ├── validation.rs         # 输入验证
│   └── audit.rs              # 安全审计
├── plugins/                  # 插件系统 (可选)
│   ├── mod.rs                # 模块声明
│   ├── manager.rs            # 插件管理器
│   ├── interface.rs          # 插件接口
│   ├── loader.rs             # 插件加载器
│   └── registry.rs           # 插件注册表
├── utils/                    # 工具函数模块
│   ├── mod.rs                # 模块声明
│   ├── async_utils.rs        # 异步工具
│   ├── file_utils.rs         # 文件操作工具
│   ├── string_utils.rs       # 字符串工具
│   ├── time_utils.rs         # 时间处理
│   ├── path_utils.rs         # 路径处理
│   ├── logger.rs             # 日志工具
│   ├── metrics.rs            # 性能指标
│   └── system.rs             # 系统信息
└── constants.rs              # 常量定义
```

## 3. 测试目录结构 (tests/)

```
tests/
├── common/                   # 测试公共代码
│   ├── mod.rs                # 模块声明
│   ├── fixtures.rs           # 测试数据固件
│   ├── mocks.rs              # Mock对象
│   ├── helpers.rs            # 测试辅助函数
│   └── setup.rs              # 测试环境设置
├── integration/              # 集成测试
│   ├── git_integration.rs    # Git集成测试
│   ├── ai_integration.rs     # AI集成测试
│   ├── agent_integration.rs  # Agent系统集成测试
│   ├── mcp_integration.rs    # MCP协议集成测试
│   ├── ui_integration.rs     # UI集成测试
│   ├── search_integration.rs # 搜索集成测试
│   └── end_to_end.rs         # 端到端测试
├── unit/                     # 单元测试
│   ├── app_tests.rs          # 应用核心测试
│   ├── git_tests.rs          # Git模块测试
│   ├── ai_tests.rs           # AI模块测试
│   ├── agent_tests.rs        # Agent系统测试
│   ├── mcp_tests.rs          # MCP协议测试
│   ├── search_tests.rs       # 搜索模块测试
│   └── ui_tests.rs           # UI模块测试
├── performance/              # 性能测试
│   ├── benchmark.rs          # 基准测试
│   ├── memory_tests.rs       # 内存使用测试
│   └── load_tests.rs         # 负载测试
├── fixtures/                 # 测试固件数据
│   ├── git_repos/            # 测试Git仓库
│   │   ├── simple_repo/      # 简单仓库
│   │   ├── large_repo/       # 大型仓库
│   │   └── complex_repo/     # 复杂仓库
│   ├── config_files/         # 测试配置文件
│   ├── mock_responses/       # 模拟API响应
│   └── sample_data/          # 示例数据
└── snapshots/                # 快照测试数据
    ├── ui_snapshots/         # UI快照
    └── output_snapshots/     # 输出快照
```

## 4. 性能测试目录 (benches/)

```
benches/
├── git_operations.rs         # Git操作性能测试
├── ui_rendering.rs           # UI渲染性能测试
├── search_performance.rs     # 搜索性能测试
├── ai_response_time.rs       # AI响应时间测试
├── agent_performance.rs      # Agent系统性能测试
├── mcp_throughput.rs         # MCP协议吞吐量测试
├── memory_usage.rs           # 内存使用测试
└── startup_time.rs           # 启动时间测试
```

## 5. 示例代码目录 (examples/)

```
examples/
├── basic_usage.rs            # 基础使用示例
├── custom_config.rs          # 自定义配置示例
├── plugin_example.rs         # 插件开发示例
├── ai_integration.rs         # AI集成示例
├── agent_development.rs      # Agent开发示例
├── mcp_server.rs             # MCP服务器示例
├── mcp_client.rs             # MCP客户端示例
├── advanced_search.rs        # 高级搜索示例
└── headless_mode.rs          # 无界面模式示例
```

## 6. 文档目录 (docs/)

```
docs/
├── README.md                 # 文档索引
├── installation.md           # 安装指南
├── user_guide.md             # 用户指南
├── configuration.md          # 配置说明
├── api_reference.md          # API参考
├── plugin_development.md     # 插件开发指南
├── agent_development.md      # Agent开发指南
├── mcp_integration.md        # MCP集成指南
├── contributing.md           # 贡献指南
├── architecture.md           # 架构说明
├── performance.md            # 性能优化
├── troubleshooting.md        # 故障排除
├── changelog.md              # 更新日志
├── Agent架构技术设计文档.md  # Agent系统架构和实现规范
├── MCP协议集成技术设计文档.md # MCP协议集成设计文档
├── 核心接口和数据结构定义文档.md # 核心接口和数据结构定义
├── 开发任务列表.md           # 开发任务列表和时间规划
├── Rust依赖包列表文档.md     # Rust依赖包完整清单
├── Rust项目文件布局文档.md   # 项目文件结构设计
├── 单元测试文档.md           # 测试策略和实施方案
├── images/                   # 文档图片
│   ├── screenshots/          # 截图
│   └── diagrams/             # 架构图
└── specs/                    # 规格说明
    ├── requirements.md       # 需求规格
    ├── design.md             # 设计文档
    ├── agent_specs.md        # Agent规格说明
    ├── mcp_specs.md          # MCP协议规格
    └── testing.md            # 测试计划
```

## 7. 资源目录 (assets/)

```
assets/
├── themes/                   # 主题文件
│   ├── dark.toml             # 深色主题
│   ├── light.toml            # 浅色主题
│   └── custom.toml           # 自定义主题模板
├── syntax/                   # 语法高亮定义
│   ├── rust.sublime-syntax   # Rust语法
│   ├── javascript.sublime-syntax # JavaScript语法
│   └── python.sublime-syntax # Python语法
├── icons/                    # 图标文件
│   ├── file_icons/           # 文件类型图标
│   └── status_icons/         # 状态图标
└── sounds/                   # 音效文件 (可选)
    ├── notification.wav      # 通知音效
    └── error.wav             # 错误音效
```

## 8. 配置文件目录 (config/)

```
config/
├── default.toml              # 默认配置
├── development.toml          # 开发环境配置
├── production.toml           # 生产环境配置
├── keybindings/              # 键盘绑定配置
│   ├── default.toml          # 默认键盘绑定
│   ├── vim.toml              # Vim风格绑定
│   └── emacs.toml            # Emacs风格绑定
└── ai_providers/             # AI服务配置
    ├── openai.toml           # OpenAI配置
    ├── claude.toml           # Claude配置
    ├── gemini.toml           # Gemini配置
    └── mcp/                  # MCP配置
        ├── servers.toml      # MCP服务器配置
        ├── tools.toml        # MCP工具配置
        └── resources.toml    # MCP资源配置
```

## 9. 脚本目录 (scripts/)

```
scripts/
├── build.sh                  # 构建脚本
├── test.sh                   # 测试脚本
├── lint.sh                   # 代码检查脚本
├── format.sh                 # 代码格式化脚本
├── release.sh                # 发布脚本
├── install.sh                # 安装脚本
├── cross_compile.sh          # 交叉编译脚本
├── ci/                       # CI/CD脚本
│   ├── build.yml             # GitHub Actions构建
│   ├── test.yml              # GitHub Actions测试
│   └── release.yml           # GitHub Actions发布
└── development/              # 开发辅助脚本
    ├── setup_env.sh          # 环境设置
    ├── generate_docs.sh      # 文档生成
    └── check_deps.sh         # 依赖检查
```

## 10. 配置文件详细说明

### 10.1 Cargo.toml 配置
```toml
[package]
name = "ai-commit"
version = "0.1.0"
edition = "2021"
rust-version = "1.70"
authors = ["Your Name <email@example.com>"]
description = "AI-powered Git commit tool with TUI interface"
documentation = "https://docs.rs/ai-commit"
repository = "https://github.com/username/ai-commit"
license = "MIT OR Apache-2.0"
keywords = ["git", "ai", "tui", "commit", "cli"]
categories = ["command-line-utilities", "development-tools"]
readme = "README.md"
exclude = [
    "target/",
    ".git/",
    "*.log",
    "screenshots/",
]

[workspace]
members = [
    "crates/ai-commit-core",
    "crates/ai-commit-git",
    "crates/ai-commit-ui",
    "crates/ai-commit-ai",
    "crates/ai-commit-mcp",
]

[[bin]]
name = "ai-commit"
path = "src/main.rs"

[lib]
name = "ai_commit"
path = "src/lib.rs"
```

### 10.2 .gitignore 配置
```gitignore
# Rust
target/
Cargo.lock
*.pdb

# IDE
.vscode/
.idea/
*.swp
*.swo
*~

# OS
.DS_Store
Thumbs.db

# Logs
*.log
logs/

# Environment
.env
.env.local

# Cache
cache/
.cache/

# Temporary files
tmp/
temp/
*.tmp

# User data
user_data/
local_config.toml

# Test artifacts
test_output/
coverage/
```

### 10.3 rustfmt.toml 配置
```toml
max_width = 100
tab_spaces = 4
newline_style = "Unix"
use_small_heuristics = "Default"
reorder_imports = true
reorder_modules = true
remove_nested_parens = true
merge_derives = true
use_try_shorthand = true
use_field_init_shorthand = true
force_explicit_abi = true
format_strings = true
format_macro_matchers = true
format_macro_bodies = true
hex_literal_case = "Upper"
```

### 10.4 clippy.toml 配置
```toml
# Clippy配置
avoid-breaking-exported-api = false
msrv = "1.70"

# 禁用的lint
disallowed-methods = []
disallowed-types = []
```

## 11. 工作空间结构 (可选)

如果项目变得复杂，可以考虑使用Cargo工作空间：

```
ai-commit-workspace/
├── Cargo.toml                # 工作空间配置
├── README.md
├── crates/                   # 子项目
│   ├── ai-commit/            # 主应用
│   │   ├── Cargo.toml
│   │   └── src/
│   ├── ai-commit-core/       # 核心功能
│   │   ├── Cargo.toml
│   │   └── src/
│   ├── ai-commit-git/        # Git操作
│   │   ├── Cargo.toml
│   │   └── src/
│   ├── ai-commit-ui/         # UI组件
│   │   ├── Cargo.toml
│   │   └── src/
│   ├── ai-commit-ai/         # AI功能 (Agent架构)
│   │   ├── Cargo.toml
│   │   └── src/
│   └── ai-commit-mcp/        # MCP协议支持
│       ├── Cargo.toml
│       └── src/
├── docs/                     # 共享文档
├── assets/                   # 共享资源
└── scripts/                  # 构建脚本
```

这个文件布局设计确保了项目的良好组织结构、代码的可维护性和团队协作的效率。每个目录和文件都有明确的职责，便于开发、测试和维护。

## 12. Agent架构和MCP集成说明

### 12.1 Agent架构设计
- **Agent管理器**: 负责Agent的生命周期管理、任务分发和结果收集
- **消息总线**: 实现Agent间的异步通信，支持发布-订阅模式
- **专用Agent**: 每个Agent负责特定功能，如提交消息生成、代码分析等
- **工具集**: 为Agent提供标准化的工具接口，包括Git操作、文件处理等

### 12.2 MCP协议支持
- **协议层**: 实现完整的MCP协议栈，支持资源、工具、提示等核心概念
- **传输层**: 支持多种传输方式（WebSocket、HTTP、标准IO）
- **处理器**: 针对不同类型的MCP消息提供专门的处理逻辑
- **扩展性**: 为未来的MCP规范更新预留扩展空间

### 12.3 配置管理
- **分离配置**: AI服务、Agent和MCP各自独立配置
- **环境适配**: 支持开发、测试、生产等不同环境配置
- **热重载**: 配置变更时无需重启应用

### 12.4 开发工作流集成
- **Agent开发**: 提供标准化的Agent开发模板和生命周期管理
- **MCP扩展**: 支持自定义MCP工具和资源的开发
- **测试策略**: 针对Agent和MCP的专门测试框架
- **文档生成**: 自动生成Agent和MCP的API文档

### 12.5 部署和运维
- **容器化**: 支持Docker容器化部署，包含MCP服务
- **监控集成**: Agent性能监控和MCP连接状态监控
- **日志聚合**: 统一的日志格式，便于问题排查
- **配置管理**: 支持多环境配置和动态配置更新

### 12.6 未来扩展规划
- **多语言支持**: 为Agent提供多编程语言接口
- **分布式架构**: 支持Agent的分布式部署和执行
- **插件生态**: 基于MCP的插件市场和生态系统
- **AI模型集成**: 支持更多AI模型和服务的无缝集成

这种架构设计为AI功能提供了更好的模块化、可扩展性和与外部AI服务的集成能力。