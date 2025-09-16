# AI-Commit Agent System Workflows

## 1. Agent生命周期管理流程

```mermaid
stateDiagram-v2
    [*] --> Uninitialized
    Uninitialized --> Initializing : register_agent()
    Initializing --> Idle : initialize() success
    Initializing --> Error : initialize() failed
    Idle --> Processing : receive_task()
    Processing --> Idle : task_complete()
    Processing --> Error : task_failed()
    Error --> Idle : recovery_success()
    Error --> Shutdown : max_failures_reached()
    Idle --> Shutting : shutdown_request()
    Processing --> Shutting : shutdown_request()
    Shutting --> Shutdown : cleanup_complete()
    Shutdown --> [*]
    Error --> [*] : unrecoverable_error()

    note right of Processing
        执行任务期间:
        - 更新健康状态
        - 报告进度
        - 处理消息
    end note

    note right of Error
        错误状态处理:
        - 触发故障恢复
        - 记录错误信息
        - 更新断路器状态
    end note
```

## 2. 任务调度与执行流程

```mermaid
flowchart TD
    START([用户请求]) --> VALIDATE{验证请求}
    VALIDATE -->|无效| ERROR_RESP[返回错误]
    VALIDATE -->|有效| CREATE_TASK[创建AgentTask]

    CREATE_TASK --> SUBMIT[提交到TaskScheduler]
    SUBMIT --> QUEUE[加入优先级队列]

    QUEUE --> SCHEDULE[调度器选择策略]
    SCHEDULE --> STRATEGY{调度策略}

    STRATEGY -->|负载均衡| LB_SELECT[选择负载最低Agent]
    STRATEGY -->|优先级| PRI_SELECT[选择最适合优先级Agent]
    STRATEGY -->|能力匹配| CAP_SELECT[选择能力最匹配Agent]

    LB_SELECT --> CHECK_AVAILABLE
    PRI_SELECT --> CHECK_AVAILABLE
    CAP_SELECT --> CHECK_AVAILABLE

    CHECK_AVAILABLE{Agent可用?}
    CHECK_AVAILABLE -->|否| WAIT[等待或重新调度]
    CHECK_AVAILABLE -->|是| ASSIGN[分配任务]

    WAIT --> SCHEDULE

    ASSIGN --> SEND_MSG[通过MessageBus发送]
    SEND_MSG --> AGENT_RECV[Agent接收任务]

    AGENT_RECV --> EXEC[Agent执行任务]
    EXEC --> HEALTH_CHECK{健康检查}

    HEALTH_CHECK -->|异常| FAILURE[触发故障处理]
    HEALTH_CHECK -->|正常| CONTINUE[继续执行]

    CONTINUE --> COMPLETE{任务完成?}
    COMPLETE -->|是| SUCCESS[返回结果]
    COMPLETE -->|否| CONTINUE

    SUCCESS --> UPDATE_METRICS[更新性能指标]
    UPDATE_METRICS --> RESPONSE[返回给用户]

    FAILURE --> RECOVERY[故障恢复流程]
    RECOVERY --> RETRY{重试?}
    RETRY -->|是| ASSIGN
    RETRY -->|否| FAIL_RESP[返回失败]

    ERROR_RESP --> END([结束])
    RESPONSE --> END
    FAIL_RESP --> END

    classDef process fill:#e1f5fe,stroke:#01579b
    classDef decision fill:#fff3e0,stroke:#ef6c00
    classDef error fill:#ffebee,stroke:#c62828
    classDef success fill:#e8f5e8,stroke:#2e7d32

    class CREATE_TASK,SUBMIT,QUEUE,ASSIGN,EXEC,UPDATE_METRICS process
    class VALIDATE,STRATEGY,CHECK_AVAILABLE,HEALTH_CHECK,COMPLETE,RETRY decision
    class ERROR_RESP,FAILURE,FAIL_RESP error
    class SUCCESS,RESPONSE success
```

## 3. 健康监控与故障恢复流程

```mermaid
flowchart TD
    subgraph "健康监控循环"
        MONITOR_START([开始监控]) --> GET_AGENTS[获取所有Agent列表]
        GET_AGENTS --> HEALTH_CHECK[执行健康检查]

        HEALTH_CHECK --> CHECK_RESULT{检查结果}
        CHECK_RESULT -->|健康| UPDATE_HEALTHY[更新为健康状态]
        CHECK_RESULT -->|降级| UPDATE_DEGRADED[更新为降级状态]
        CHECK_RESULT -->|不健康| UPDATE_UNHEALTHY[更新为不健康状态]

        UPDATE_HEALTHY --> RESET_FAILURES[重置失败计数]
        UPDATE_DEGRADED --> LOG_WARNING[记录警告]
        UPDATE_UNHEALTHY --> INCREMENT_FAILURES[增加失败计数]

        RESET_FAILURES --> NEXT_AGENT
        LOG_WARNING --> NEXT_AGENT
        INCREMENT_FAILURES --> CHECK_THRESHOLD{超过阈值?}

        CHECK_THRESHOLD -->|否| NEXT_AGENT[下一个Agent]
        CHECK_THRESHOLD -->|是| TRIGGER_RECOVERY[触发故障恢复]

        NEXT_AGENT --> MORE_AGENTS{还有Agent?}
        MORE_AGENTS -->|是| HEALTH_CHECK
        MORE_AGENTS -->|否| WAIT_INTERVAL[等待检查间隔]
        WAIT_INTERVAL --> GET_AGENTS
    end

    subgraph "故障恢复流程"
        TRIGGER_RECOVERY --> CREATE_FAILURE_INFO[创建FailureInfo]
        CREATE_FAILURE_INFO --> CHECK_CIRCUIT{断路器状态}

        CHECK_CIRCUIT -->|开启| BLOCK_REQUEST[阻止请求]
        CHECK_CIRCUIT -->|关闭/半开| ALLOW_RECOVERY[允许恢复]

        BLOCK_REQUEST --> WAIT_TIMEOUT[等待超时]
        WAIT_TIMEOUT --> HALF_OPEN[切换到半开状态]
        HALF_OPEN --> ALLOW_RECOVERY

        ALLOW_RECOVERY --> SELECT_STRATEGY[选择恢复策略]
        SELECT_STRATEGY --> STRATEGY_TYPE{策略类型}

        STRATEGY_TYPE -->|重启| RESTART_AGENT[重启Agent]
        STRATEGY_TYPE -->|降级| DEGRADE_AGENT[切换降级模式]
        STRATEGY_TYPE -->|故障转移| FAILOVER_AGENT[故障转移]

        RESTART_AGENT --> RECOVERY_RESULT{恢复结果}
        DEGRADE_AGENT --> RECOVERY_RESULT
        FAILOVER_AGENT --> RECOVERY_RESULT

        RECOVERY_RESULT -->|成功| RECOVERY_SUCCESS[恢复成功]
        RECOVERY_RESULT -->|失败| TRY_NEXT_STRATEGY[尝试下一策略]

        RECOVERY_SUCCESS --> CLOSE_CIRCUIT[关闭断路器]
        RECOVERY_SUCCESS --> SEND_ALERT[发送恢复通知]

        TRY_NEXT_STRATEGY --> MORE_STRATEGIES{还有策略?}
        MORE_STRATEGIES -->|是| SELECT_STRATEGY
        MORE_STRATEGIES -->|否| RECOVERY_FAILED[恢复失败]

        RECOVERY_FAILED --> OPEN_CIRCUIT[打开断路器]
        RECOVERY_FAILED --> SEND_FAILURE_ALERT[发送失败警报]

        CLOSE_CIRCUIT --> MONITOR_CONTINUE[继续监控]
        OPEN_CIRCUIT --> MONITOR_CONTINUE
        SEND_ALERT --> MONITOR_CONTINUE
        SEND_FAILURE_ALERT --> MONITOR_CONTINUE

        MONITOR_CONTINUE --> NEXT_AGENT
    end

    classDef monitor fill:#e8f5e8,stroke:#2e7d32
    classDef recovery fill:#fff3e0,stroke:#ef6c00
    classDef circuit fill:#e1f5fe,stroke:#01579b
    classDef alert fill:#fce4ec,stroke:#c2185b

    class GET_AGENTS,HEALTH_CHECK,UPDATE_HEALTHY,UPDATE_DEGRADED,UPDATE_UNHEALTHY monitor
    class CREATE_FAILURE_INFO,SELECT_STRATEGY,RESTART_AGENT,DEGRADE_AGENT,FAILOVER_AGENT recovery
    class CHECK_CIRCUIT,BLOCK_REQUEST,HALF_OPEN,CLOSE_CIRCUIT,OPEN_CIRCUIT circuit
    class SEND_ALERT,SEND_FAILURE_ALERT alert
```

## 4. 消息总线通信流程

```mermaid
sequenceDiagram
    participant AM as Agent Manager
    participant MB as Message Bus
    participant MR as Message Router
    participant A1 as Agent 1
    participant A2 as Agent 2
    participant HM as Health Monitor

    Note over MB: 初始化消息总线
    AM->>MB: register_agent(agent1_id, channel1)
    MB->>MR: 注册路由表
    AM->>MB: register_agent(agent2_id, channel2)
    MB->>MR: 更新路由表

    Note over AM: 发送点对点消息
    AM->>MB: send_message(task_assignment)
    MB->>MR: route_message(target_agent)
    MR->>MR: 查找目标Agent通道
    MR->>A1: 发送任务消息
    A1->>A1: 处理任务
    A1->>MB: send_message(task_result)
    MB->>MR: route_message(agent_manager)
    MR->>AM: 返回任务结果

    Note over HM: 广播健康检查
    HM->>MB: broadcast(health_check_request)
    MB->>A1: 广播消息
    MB->>A2: 广播消息
    A1->>MB: send_message(health_status)
    A2->>MB: send_message(health_status)
    MB->>HM: 收集健康状态

    Note over MB: 处理Agent下线
    AM->>MB: unregister_agent(agent1_id)
    MB->>MR: 从路由表移除
    MB->>MB: 清理相关资源

    Note over MB: 性能监控
    loop 每10ms
        MB->>MB: 统计消息吞吐量
        MB->>MB: 监控路由延迟
        MB->>MB: 检查通道健康状态
    end
```

## 5. AI服务集成流程

```mermaid
flowchart TD
    START([AI服务请求]) --> SELECT_PROVIDER{选择AI Provider}

    SELECT_PROVIDER -->|OpenAI| OPENAI_CLIENT[OpenAI Client]
    SELECT_PROVIDER -->|Anthropic| ANTHROPIC_CLIENT[Anthropic Client]
    SELECT_PROVIDER -->|Ollama| OLLAMA_CLIENT[Ollama Client]
    SELECT_PROVIDER -->|Custom| CUSTOM_CLIENT[Custom Client]

    OPENAI_CLIENT --> BUILD_REQUEST[构建API请求]
    ANTHROPIC_CLIENT --> BUILD_REQUEST
    OLLAMA_CLIENT --> BUILD_REQUEST
    CUSTOM_CLIENT --> BUILD_REQUEST

    BUILD_REQUEST --> GET_PROMPT[获取Prompt模板]
    GET_PROMPT --> RENDER_PROMPT[渲染Prompt]

    RENDER_PROMPT --> SET_PARAMS[设置请求参数]
    SET_PARAMS --> SEND_REQUEST[发送HTTP请求]

    SEND_REQUEST --> TIMEOUT_CHECK{请求超时?}
    TIMEOUT_CHECK -->|是| TIMEOUT_ERROR[超时错误]
    TIMEOUT_CHECK -->|否| RESPONSE_CHECK{响应状态}

    RESPONSE_CHECK -->|成功| PARSE_RESPONSE[解析响应]
    RESPONSE_CHECK -->|失败| HTTP_ERROR[HTTP错误]

    PARSE_RESPONSE --> VALIDATE_RESPONSE{验证响应格式}
    VALIDATE_RESPONSE -->|有效| EXTRACT_CONTENT[提取内容]
    VALIDATE_RESPONSE -->|无效| FORMAT_ERROR[格式错误]

    EXTRACT_CONTENT --> UPDATE_STATS[更新统计信息]
    UPDATE_STATS --> CACHE_RESULT[缓存结果]
    CACHE_RESULT --> RETURN_SUCCESS[返回成功结果]

    TIMEOUT_ERROR --> RETRY_CHECK{是否重试?}
    HTTP_ERROR --> RETRY_CHECK
    FORMAT_ERROR --> RETRY_CHECK

    RETRY_CHECK -->|是| RETRY_DELAY[延迟重试]
    RETRY_CHECK -->|否| RETURN_ERROR[返回错误]

    RETRY_DELAY --> BUILD_REQUEST

    RETURN_SUCCESS --> END([结束])
    RETURN_ERROR --> END

    classDef client fill:#fff3e0,stroke:#ef6c00
    classDef process fill:#e1f5fe,stroke:#01579b
    classDef error fill:#ffebee,stroke:#c62828
    classDef success fill:#e8f5e8,stroke:#2e7d32

    class OPENAI_CLIENT,ANTHROPIC_CLIENT,OLLAMA_CLIENT,CUSTOM_CLIENT client
    class BUILD_REQUEST,GET_PROMPT,RENDER_PROMPT,SET_PARAMS,PARSE_RESPONSE,UPDATE_STATS process
    class TIMEOUT_ERROR,HTTP_ERROR,FORMAT_ERROR,RETURN_ERROR error
    class EXTRACT_CONTENT,CACHE_RESULT,RETURN_SUCCESS success
```

## 6. Git集成与分析流程

```mermaid
flowchart TD
    START([Git分析请求]) --> FIND_ROOT[查找Git根目录]
    FIND_ROOT --> CHECK_REPO{是否为Git仓库?}

    CHECK_REPO -->|否| NO_REPO_ERROR[非Git仓库错误]
    CHECK_REPO -->|是| GET_STATUS[获取Git状态]

    GET_STATUS --> PARSE_STATUS[解析工作区状态]
    PARSE_STATUS --> CHECK_CHANGES{有变更文件?}

    CHECK_CHANGES -->|否| NO_CHANGES[无变更提示]
    CHECK_CHANGES -->|是| GET_DIFF[获取文件差异]

    GET_DIFF --> ANALYZE_DIFF[分析差异内容]
    ANALYZE_DIFF --> CATEGORIZE_CHANGES[分类变更类型]

    CATEGORIZE_CHANGES --> CHANGE_TYPE{变更类型}
    CHANGE_TYPE -->|新增| NEW_FILES[处理新增文件]
    CHANGE_TYPE -->|修改| MODIFIED_FILES[处理修改文件]
    CHANGE_TYPE -->|删除| DELETED_FILES[处理删除文件]
    CHANGE_TYPE -->|重命名| RENAMED_FILES[处理重命名文件]

    NEW_FILES --> EXTRACT_INFO[提取文件信息]
    MODIFIED_FILES --> EXTRACT_INFO
    DELETED_FILES --> EXTRACT_INFO
    RENAMED_FILES --> EXTRACT_INFO

    EXTRACT_INFO --> GET_BRANCH[获取当前分支]
    GET_BRANCH --> GET_COMMITS[获取提交历史]
    GET_COMMITS --> ANALYZE_PATTERN[分析提交模式]

    ANALYZE_PATTERN --> BUILD_CONTEXT[构建上下文信息]
    BUILD_CONTEXT --> VALIDATE_DATA{验证数据完整性}

    VALIDATE_DATA -->|无效| DATA_ERROR[数据错误]
    VALIDATE_DATA -->|有效| RETURN_ANALYSIS[返回分析结果]

    NO_REPO_ERROR --> END([结束])
    NO_CHANGES --> END
    DATA_ERROR --> END
    RETURN_ANALYSIS --> END

    classDef git fill:#fce4ec,stroke:#c2185b
    classDef analysis fill:#e1f5fe,stroke:#01579b
    classDef error fill:#ffebee,stroke:#c62828
    classDef success fill:#e8f5e8,stroke:#2e7d32

    class FIND_ROOT,GET_STATUS,GET_DIFF,GET_BRANCH,GET_COMMITS git
    class PARSE_STATUS,ANALYZE_DIFF,CATEGORIZE_CHANGES,ANALYZE_PATTERN,BUILD_CONTEXT analysis
    class NO_REPO_ERROR,DATA_ERROR error
    class EXTRACT_INFO,RETURN_ANALYSIS success
```

## 7. 完整的提交消息生成流程

```mermaid
sequenceDiagram
    participant U as User
    participant UI as TUI Interface
    participant AM as Agent Manager
    participant TS as Task Scheduler
    participant MB as Message Bus
    participant CA as Commit Agent
    participant GS as Git Service
    participant AIC as AI Client
    participant HM as Health Monitor
    participant FR as Failure Recovery

    Note over U,FR: 提交消息生成完整流程

    U->>UI: 启动ai-commit命令
    UI->>AM: 初始化Agent系统
    AM->>HM: 启动健康监控
    AM->>CA: 注册CommitAgent
    CA->>MB: 注册消息通道

    U->>UI: 请求生成提交消息
    UI->>GS: 检查Git仓库状态
    GS->>UI: 返回仓库信息

    alt Git仓库无变更
        UI->>U: 提示无变更文件
    else 有变更文件
        UI->>AM: 创建提交消息生成任务
        AM->>TS: 提交任务到调度器
        TS->>TS: 选择CommitAgent (能力匹配)
        TS->>MB: 发送任务分配消息
        MB->>CA: 路由消息到CommitAgent

        CA->>GS: 获取详细Git状态
        GS->>CA: 返回staged files和diff
        CA->>AIC: 调用AI服务生成消息

        alt AI服务调用成功
            AIC->>CA: 返回生成的提交消息
            CA->>HM: 更新健康状态 (成功)
            CA->>MB: 发送任务完成消息
            MB->>AM: 路由结果到AgentManager
            AM->>UI: 返回提交消息
            UI->>U: 显示生成的提交消息
        else AI服务调用失败
            AIC->>CA: 返回错误信息
            CA->>HM: 更新健康状态 (失败)
            HM->>FR: 触发故障恢复
            FR->>FR: 执行恢复策略

            alt 恢复成功
                FR->>CA: 通知恢复完成
                CA->>AIC: 重试AI服务调用
                AIC->>CA: 返回生成的提交消息
                CA->>MB: 发送任务完成消息
                MB->>AM: 路由结果到AgentManager
                AM->>UI: 返回提交消息
                UI->>U: 显示生成的提交消息
            else 恢复失败
                FR->>AM: 通知恢复失败
                AM->>UI: 返回错误信息
                UI->>U: 显示错误提示
            end
        end
    end

    Note over U,FR: 流程结束，等待下次请求
```

## 性能指标与监控点

### 关键性能指标 (KPIs)

| 指标 | 目标值 | 监控方法 | 告警阈值 |
|-----|--------|---------|---------|
| 应用启动时间 | < 1秒 | 启动计时器 | > 2秒 |
| Agent初始化时间 | < 500ms | Agent生命周期计时 | > 1秒 |
| 任务调度延迟 | < 10ms | 任务队列监控 | > 50ms |
| 消息路由延迟 | < 10ms | 消息总线统计 | > 20ms |
| 健康检查响应 | < 100ms | 健康监控计时 | > 200ms |
| AI服务响应时间 | < 5秒 | AI客户端统计 | > 10秒 |
| 内存使用率 | < 80% | 系统监控 | > 90% |
| 任务成功率 | > 95% | 任务执行统计 | < 90% |

### 监控数据收集点

```mermaid
graph LR
    subgraph "性能监控数据流"
        APP[应用层] --> METRICS[指标收集器]
        AGENT[Agent层] --> METRICS
        SCHEDULER[调度器] --> METRICS
        MESSAGEBUS[消息总线] --> METRICS
        HEALTH[健康监控] --> METRICS
        AI[AI服务] --> METRICS
        GIT[Git服务] --> METRICS

        METRICS --> STORAGE[(时序数据库)]
        STORAGE --> DASHBOARD[监控面板]
        STORAGE --> ALERT[告警系统]

        DASHBOARD --> ADMIN[管理员]
        ALERT --> ADMIN
    end
```

这个架构文档提供了AI-Commit Agent系统的完整视图，包括系统架构、数据流、错误处理、性能监控等各个方面的详细设计。