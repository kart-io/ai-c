# AI-C TUI 使用指南

## 安装

```bash
# 克隆项目
git clone <repository-url>
cd ai-c

# 安装应用
make install
```

## 使用方式

### 1. 完整 TUI 模式（推荐在正常终端中使用）

```bash
ai-c
```

- 在Git仓库中运行以获得完整功能
- 提供交互式终端UI界面
- 支持文件状态查看、分支管理等功能

### 2. 演示模式（适合在受限环境中使用）

```bash
ai-c --demo
```

- 显示应用功能概览和演示信息
- 不需要完整的终端支持
- 适合在Claude Code等环境中使用

### 3. 环境变量控制

```bash
# 强制演示模式
AI_C_DEMO_MODE=1 ai-c
```

## UI界面功能

### 主要标签页

- **Status Tab** - 文件更改状态和暂存区管理
- **Branches Tab** - Git分支管理
- **Tags Tab** - Git标签管理
- **Stash Tab** - Git暂存操作
- **Remotes Tab** - 远程仓库管理
- **GitFlow Tab** - GitFlow工作流操作

### 键盘快捷键

- **Tab** - 在不同标签页之间切换
- **方向键** 或 **j/k** - 在列表项中导航
- **空格键** - 切换选择状态
- **q** 或 **Esc** - 退出应用

## 故障排除

### 应用无法启动TUI界面

如果遇到"Device not configured"或其他终端错误：

1. 尝试使用演示模式：`ai-c --demo`
2. 检查终端是否支持TUI应用
3. 确认在支持的终端环境中运行

### 非Git目录

- 应用会自动检测是否在Git仓库中
- 在非Git目录中会启用模拟模式
- 所有UI功能仍然可用，但数据为模拟数据

## 开发命令

```bash
# 构建项目
make build

# 运行测试
make test

# 运行开发模式
make run

# 代码格式化
make fmt

# 代码检查
make clippy
```

## 性能特性

- 启动时间 < 1秒
- Git状态检查 < 200ms
- 支持 >10,000 文件的大型仓库
- 内存使用 < 150MB（空闲状态）

## 系统要求

- Rust 1.70+
- Git 2.0+
- 支持TUI的终端（用于完整模式）