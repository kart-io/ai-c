# 布局问题调试指南

## 问题描述
- Tabs组件占据整个顶部导航栏宽度
- Sync status组件显示在主内容区右上角

## 预期行为
- Tabs组件在左侧，Sync status在右侧，都在顶部3行高的导航栏内

## 调试步骤

### 1. 验证终端大小
确保终端宽度至少 > 50 字符，以便有足够空间显示分割布局

### 2. 检查当前代码版本
确认运行的是最新修复后的代码：
```bash
# 重新构建确保使用最新代码
cargo build --release

# 运行程序
./target/release/ai-c
```

### 3. 调试布局约束
如果问题仍然存在，可以临时修改 `src/ui/mod.rs` 行213-216:

```rust
// 原始代码
.constraints([
    Constraint::Min(0),     // Tabs area
    Constraint::Length(30), // Sync status area
])

// 调试代码 - 使用百分比约束
.constraints([
    Constraint::Percentage(70), // Tabs area - 70%
    Constraint::Percentage(30), // Sync status area - 30%
])
```

### 4. 增加边框可见性
临时修改 `render_sync_status` 函数，使边框更明显：

```rust
// 在 src/ui/mod.rs 的 render_sync_status 函数中
.border_style(Style::default().fg(Color::Red)) // 临时使用红色边框
```

### 5. 验证布局分割
在 `render_header_nav` 函数中添加调试信息：

```rust
// 在行217之后添加
debug!("Header area: {:?}", area);
debug!("Header layout[0] (tabs): {:?}", header_layout[0]);
debug!("Header layout[1] (sync): {:?}", header_layout[1]);
```

## 可能的原因

1. **约束冲突**: `Constraint::Min(0)` 可能在某些情况下占用全部空间
2. **主题颜色**: 边框颜色太暗，视觉上看不出分割
3. **ratatui版本**: 不同版本的布局行为可能略有差异

## 推荐解决方案

如果问题持续存在，建议：
1. 使用百分比约束替代 Min/Length 约束
2. 增加明显的边框样式用于调试
3. 添加日志输出验证布局计算结果