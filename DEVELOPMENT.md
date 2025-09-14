# AI-C TUI Development Guide

本文档介绍如何使用项目的开发工具和命令。

## 快速开始

```bash
# 查看所有可用命令
make help

# 设置开发环境 (首次使用)
make dev-setup

# 快速开发流程
make all          # 完整构建流程：清理 -> 格式化 -> 检查 -> 测试 -> 构建
```

## 核心开发命令

### 构建项目

```bash
make build          # Debug 构建
make build-release  # Release 构建
make rebuild        # 清理后重新构建
```

### 运行项目

```bash
make run           # 运行 Debug 版本
make run-release   # 运行 Release 版本
make run-verbose   # 带详细日志运行
```

### 测试

```bash
make test          # 运行所有测试
make test-unit     # 仅单元测试
make test-core     # 核心功能测试
make test-watch    # 监视模式运行测试
```

### 代码质量

```bash
make fmt           # 格式化代码
make clippy        # 运行 Clippy 检查
make fix           # 自动修复问题
make pre-commit    # 预提交检查
```

## 高级命令

### 文档生成

```bash
make doc           # 生成文档
make doc-open      # 生成并打开文档
```

### 性能分析

```bash
make bench         # 运行基准测试
make coverage      # 生成测试覆盖率
make perf-test     # 性能测试
```

### 依赖管理

```bash
make deps          # 显示依赖树
make deps-update   # 更新依赖
make deps-audit    # 安全审计
```

### 实用工具

```bash
make size          # 显示二进制文件大小
make lines         # 统计代码行数
make todos         # 查找 TODO/FIXME
```

## 开发工作流程

### 日常开发

1. **开始工作**：
   ```bash
   make check         # 快速检查
   make test-watch    # 后台运行测试
   ```

2. **提交前**：
   ```bash
   make pre-commit    # 格式化、检查、测试
   ```

3. **发布前**：
   ```bash
   make release-check # 完整的发布检查
   ```

### 问题排查

```bash
make check         # 检查编译错误
make clippy        # 查找代码问题
make test-verbose  # 详细测试输出
```

## 性能要求验证

项目有严格的性能要求，可以使用以下命令验证：

```bash
# 性能基准测试
make bench

# 特定组件测试
make bench-git     # Git 操作性能
make bench-agent   # Agent 系统性能

# 内存使用检查
make memory-check
```

## 项目信息

```bash
make info          # 显示项目基本信息
```

## 安装和部署

```bash
make install       # 安装到系统
make uninstall     # 从系统卸载
```

## 监控模式

```bash
make monitor       # 持续监控构建和测试状态
```

## 配置文件

项目包含以下配置文件：

- `rustfmt.toml` - 代码格式化配置
- `clippy.toml` - Clippy 检查配置
- `.gitignore` - Git 忽略文件配置

这些文件确保团队开发时的代码风格一致性。

## 性能目标

使用 `make bench` 验证以下性能目标：

- 应用启动时间 < 1秒
- Agent 系统初始化 < 500ms
- Git 状态刷新 < 200ms (支持 >10,000 文件)
- 消息总线路由 < 10ms
- 内存使用 < 150MB (空闲状态)

## 故障排除

### 编译问题

```bash
make clean         # 清理构建文件
make rebuild       # 重新构建
```

### 依赖问题

```bash
make deps-update   # 更新依赖
make deps-audit    # 检查安全问题
```

### 测试失败

```bash
make test-verbose  # 详细测试输出
make test-unit     # 仅运行单元测试
```