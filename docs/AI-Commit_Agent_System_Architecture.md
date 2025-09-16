# AI-Commit TUI Agent System Architecture

## 系统架构概览

```mermaid
graph TB
    subgraph "AI-Commit TUI Application"
        subgraph "Presentation Layer (TUI)"
            UI[TUI Interface]
            CLI[Command Line Interface]
        end

        subgraph "Application Layer"
            APP[App Controller]
            STATE[App State Manager]
            EVENT[Event Handler]
        end

        subgraph "Agent System Core"
            subgraph "Agent Management"
                AM[Agent Manager]
                AR[Agent Registry]
                AL[Agent Lifecycle]
            end

            subgraph "Task Processing"
                TS[Task Scheduler]
                TQ[Task Queue]
                LB[Load Balancer]
            end

            subgraph "Communication"
                MB[Message Bus]
                MR[Message Router]
                MC[Message Channels]
            end

            subgraph "Health & Recovery"
                HM[Health Monitor]
                FR[Failure Recovery]
                CB[Circuit Breaker]
            end
        end

        subgraph "Specialized Agents"
            CA[Commit Agent]
            AA[Analysis Agent]
            RA[Review Agent]
            SA[Search Agent]
        end

        subgraph "AI Services"
            AIC[AI Client]
            PM[Prompt Manager]
            subgraph "AI Providers"
                OPENAI[OpenAI]
                ANTHROPIC[Anthropic]
                OLLAMA[Ollama]
                CUSTOM[Custom Provider]
            end
        end

        subgraph "Git Integration"
            GS[Git Service]
            GO[Git Operations]
            subgraph "Git Data"
                GR[Git Repository]
                GB[Git Branches]
                GC[Git Commits]
                GD[Git Diff]
            end
        end

        subgraph "Infrastructure"
            CONFIG[Configuration]
            LOG[Logging]
            ERR[Error Handling]
        end
    end

    %% Connections
    UI --> APP
    CLI --> APP
    APP --> STATE
    APP --> EVENT

    EVENT --> AM
    AM --> TS
    AM --> HM
    AM --> FR

    TS --> TQ
    TS --> LB
    TS --> MB

    MB --> MR
    MR --> MC
    MC --> CA
    MC --> AA
    MC --> RA
    MC --> SA

    CA --> AIC
    AA --> AIC
    RA --> AIC
    SA --> AIC

    AIC --> PM
    AIC --> OPENAI
    AIC --> ANTHROPIC
    AIC --> OLLAMA
    AIC --> CUSTOM

    HM --> AM
    FR --> CB
    FR --> AM

    CA --> GS
    AA --> GS
    RA --> GS

    GS --> GO
    GO --> GR
    GO --> GB
    GO --> GC
    GO --> GD

    APP --> CONFIG
    APP --> LOG
    APP --> ERR

    classDef agentCore fill:#e1f5fe,stroke:#01579b,stroke-width:2px
    classDef agent fill:#e8f5e8,stroke:#2e7d32,stroke-width:2px
    classDef ai fill:#fff3e0,stroke:#ef6c00,stroke-width:2px
    classDef git fill:#fce4ec,stroke:#c2185b,stroke-width:2px
    classDef infra fill:#f3e5f5,stroke:#7b1fa2,stroke-width:2px

    class AM,TS,MB,HM,FR agentCore
    class CA,AA,RA,SA agent
    class AIC,PM,OPENAI,ANTHROPIC,OLLAMA,CUSTOM ai
    class GS,GO,GR,GB,GC,GD git
    class CONFIG,LOG,ERR infra
```

## 核心组件说明

### 1. Agent Management (Agent管理)
- **AgentManager**: 中央Agent管理器，负责Agent生命周期
- **AgentRegistry**: Agent注册表，维护所有已注册的Agent
- **AgentLifecycle**: Agent生命周期管理（初始化、运行、关闭）

### 2. Task Processing (任务处理)
- **TaskScheduler**: 任务调度器，实现三种调度策略
  - LoadBalancingStrategy: 负载均衡策略
  - PriorityBasedStrategy: 优先级调度策略
  - CapabilityMatchStrategy: 能力匹配策略
- **TaskQueue**: 优先级任务队列
- **LoadBalancer**: 负载均衡器

### 3. Communication (通信系统)
- **MessageBus**: 高性能消息总线
- **MessageRouter**: 消息路由器
- **MessageChannels**: Agent间通信通道

### 4. Health & Recovery (健康监控与故障恢复)
- **HealthMonitor**: 健康监控系统
  - ResponseTimeCheck: 响应时间检查
  - MemoryUsageCheck: 内存使用检查
  - 可插拔健康检查机制
- **FailureRecovery**: 故障恢复管理器
  - RestartRecoveryStrategy: 重启恢复策略
  - DegradedModeRecoveryStrategy: 降级恢复策略
  - FailoverRecoveryStrategy: 故障转移策略
- **CircuitBreaker**: 断路器模式实现

### 5. Specialized Agents (专用Agent)
- **CommitAgent**: 提交消息生成Agent
- **AnalysisAgent**: 代码分析Agent（待实现）
- **ReviewAgent**: 代码审查Agent（待实现）
- **SearchAgent**: 语义搜索Agent（待实现）

## 数据流架构

```mermaid
sequenceDiagram
    participant U as User
    participant UI as TUI Interface
    participant AM as Agent Manager
    participant TS as Task Scheduler
    participant MB as Message Bus
    participant CA as Commit Agent
    participant AIC as AI Client
    participant GS as Git Service

    U->>UI: 启动应用
    UI->>AM: 初始化Agent系统
    AM->>CA: 注册CommitAgent
    CA->>AM: 注册成功

    U->>UI: 请求生成提交消息
    UI->>AM: 创建提交消息任务
    AM->>TS: 提交任务到调度器
    TS->>TS: 选择最优Agent
    TS->>MB: 发送任务消息
    MB->>CA: 路由到CommitAgent

    CA->>GS: 获取Git状态
    GS->>CA: 返回文件变更信息
    CA->>AIC: 调用AI服务
    AIC->>AIC: 生成提交消息
    AIC->>CA: 返回生成结果

    CA->>MB: 发送任务结果
    MB->>AM: 路由结果消息
    AM->>UI: 返回生成的提交消息
    UI->>U: 显示提交消息
```

## 性能要求

| 组件 | 性能指标 | 实现状态 |
|-----|---------|---------|
| 应用启动 | < 1秒 | ✅ 已实现 |
| Agent初始化 | < 500ms | ✅ 已实现 |
| 任务处理 | < 2秒 (普通优先级) | ✅ 已实现 |
| 健康检查 | < 100ms | ✅ 已实现 |
| 消息路由 | < 10ms | ✅ 已实现 |
| 广播消息 | < 50ms | ✅ 已实现 |

## 错误处理层次

```mermaid
graph TD
    subgraph "错误处理架构"
        UE[User Errors]
        AE[Application Errors]
        AGE[Agent Errors]
        ME[Monitor Errors]
        RE[Recovery Errors]
        GE[Git Errors]
        AIE[AI Service Errors]

        UE --> AE
        AE --> AGE
        AE --> GE
        AE --> AIE
        AGE --> ME
        AGE --> RE

        ME --> FR[Failure Recovery]
        RE --> CB[Circuit Breaker]

        FR --> ALERT[Alert System]
        CB --> ALERT

        ALERT --> LOG[Logging System]
        ALERT --> UI[User Notification]
    end
```

## 扩展性设计

### 1. Agent扩展
- 实现Agent trait即可添加新Agent
- 支持运行时动态注册
- 可插拔能力系统

### 2. AI Provider扩展
- 统一AIClient接口
- 支持多种AI服务提供商
- 可配置的Provider选择

### 3. 健康检查扩展
- 实现HealthCheck trait
- 支持自定义检查逻辑
- 可配置的检查频率

### 4. 恢复策略扩展
- 实现RecoveryStrategy trait
- 支持自定义恢复逻辑
- 优先级排序执行

## 配置管理

```yaml
# ai-commit.toml 示例配置
[agent_manager]
max_agents = 50
default_task_timeout = "30s"
health_check_interval = "10s"

[task_scheduler]
max_queue_size = 1000
default_timeout = "300s"
strategy = "LoadBalancing"

[health_monitor]
check_interval = "30s"
max_failures = 3
enable_auto_recovery = true

[failure_recovery]
enable_auto_recovery = true
max_recovery_attempts = 3
recovery_interval = "60s"

[ai_client]
default_provider = "openai"
timeout = "30s"
max_retries = 3

[logging]
level = "info"
format = "json"
file = "ai-commit.log"
```

## 内存使用模式

| 组件 | 预估内存使用 | 优化策略 |
|-----|-------------|---------|
| Agent系统核心 | ~50MB | Arc + RwLock共享状态 |
| 消息队列 | ~20MB | 无界队列 + 及时清理 |
| AI客户端缓存 | ~30MB | LRU缓存 + 大小限制 |
| Git数据缓存 | ~40MB | 增量更新 + 压缩存储 |
| TUI渲染缓存 | ~10MB | 帧缓冲 + 差异更新 |
| **总计 (空闲)** | **~150MB** | 符合设计要求 |
| **总计 (活跃)** | **~600MB** | 多Agent并发时 |