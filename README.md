# AI-C TUI

一个智能的Git提交工具，基于Rust构建，具有终端用户界面(TUI)和AI Agent架构。

## ⚡ 快速开始

```bash
# 查看所有可用命令
make help

# 设置开发环境 (首次使用)
make dev-setup

# 构建项目
make build

# 运行项目
make run

# 运行测试
make test
```

## 🏗️ 项目架构

本项目采用5层模块化架构：

- **App层** - 应用程序核心和状态管理
- **UI层** - 基于ratatui的终端用户界面
- **Git层** - 高性能Git操作和缓存
- **AI层** - Agent系统和消息总线
- **Config层** - TOML配置管理

## 🛠️ 开发工具

### 基础命令
```bash
make build          # 构建项目 (debug)
make build-release  # 构建项目 (release)
make run            # 运行项目
make test           # 运行所有测试
make clean          # 清理构建文件
```

### 代码质量
```bash
make fmt            # 格式化代码
make clippy         # 运行Clippy检查
make check          # 快速代码检查
make pre-commit     # 预提交检查
```

### 测试相关
```bash
make test-unit      # 单元测试
make test-core      # 核心测试
make test-minimal   # 最小测试集
make test-watch     # 监视模式测试
```

### 高级功能
```bash
make doc            # 生成文档
make bench          # 性能基准测试
make coverage       # 测试覆盖率
make size           # 查看二进制文件大小
make lines          # 统计代码行数
```

## 📊 性能目标

- 🚀 应用启动时间: < 1秒
- ⚡ Agent系统初始化: < 500ms
- 📁 Git状态刷新: < 200ms (支持 >10,000 文件)
- 💬 消息总线路由: < 10ms
- 💾 内存使用: < 150MB (空闲状态)

## 🔧 项目特性

### 已实现 ✅
- **完整的项目结构** - 模块化架构设计
- **Git服务模块** - 高性能Git操作和智能缓存
- **TUI界面框架** - 响应式终端界面
- **Agent系统** - 基于Actor模型的AI服务
- **状态管理** - 集中式状态和事件系统
- **配置系统** - TOML配置和热重载
- **测试基础设施** - 完整测试和基准框架

### 开发中 🚧
- MCP协议集成
- 专用AI Agent实现
- 高级UI功能

## 📁 项目结构

```
ai-c/
├── Cargo.toml              # 依赖配置
├── Makefile               # 开发命令
├── src/
│   ├── main.rs           # 应用程序入口
│   ├── lib.rs            # 库导出
│   ├── app/              # 应用程序核心
│   ├── ui/               # 用户界面组件
│   ├── git/              # Git操作服务
│   ├── ai/               # AI Agent系统
│   └── config/           # 配置管理
├── tests/                # 测试文件
└── docs/                 # 文档
```

## 🚀 开发工作流

### 日常开发
1. `make check` - 快速检查
2. `make test-watch` - 后台测试监控
3. 开发...
4. `make pre-commit` - 提交前检查

### 发布流程
1. `make release-check` - 发布检查
2. `make build-release` - 构建发布版本
3. `make install` - 安装到系统

## 📖 详细文档

- `make help` - 查看所有命令
- `DEVELOPMENT.md` - 详细开发指南
- `docs/开发任务列表.md` - 开发任务清单

## 🔍 代码统计

```bash
make lines          # 查看代码行数统计
make todos          # 查找TODO和FIXME
make size           # 查看二进制大小
```

当前代码统计：**4,743行 Rust代码**

## 🛡️ 代码质量

项目包含完整的代码质量工具链：

- **rustfmt** - 代码格式化
- **clippy** - 代码检查
- **测试覆盖率** - tarpaulin支持
- **基准测试** - criterion集成
- **依赖审计** - cargo audit

## ⚙️ 系统要求

- Rust 1.70+
- Git 2.0+
- 终端支持 (支持256色)

## 🤝 贡献

1. Fork项目
2. 创建功能分支 (`git checkout -b feature/amazing-feature`)
3. 提交更改 (`git commit -m 'Add amazing feature'`)
4. 推送分支 (`git push origin feature/amazing-feature`)
5. 创建Pull Request

## 📄 许可证

本项目采用MIT许可证 - 查看 [LICENSE](LICENSE) 文件了解详情。

---

## 📚 更多信息

- **架构文档**: `docs/Agent架构技术设计文档.md`
- **MCP集成**: `docs/MCP协议集成技术设计文档.md`
- **开发指南**: `DEVELOPMENT.md`
- **项目状态**: `IMPLEMENTATION_SUMMARY.md`