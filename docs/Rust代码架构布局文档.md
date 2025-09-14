# Rust代码架构布局文档

基于AI-Commit TUI的Rust应用架构设计

## 1. 整体架构模式

### 1.1 分层架构
```
┌─────────────────────────────────────────────────────────────┐
│                    表示层 (Presentation)                     │
│  ┌─────────────────────┐    ┌─────────────────────────────┐  │
│  │   TUI Interface     │    │    Terminal Components      │  │
│  │  (ratatui + 组件)    │    │  ┌─────────┬─────────────┐  │  │
│  │                     │    │  │  File   │    Diff     │  │  │
│  │                     │    │  │  List   │   Viewer    │  │  │
│  │                     │    │  ├─────────┼─────────────┤  │  │
│  │                     │    │  │Commit   │  Branch     │  │  │
│  │                     │    │  │History  │  Manager    │  │  │
│  │                     │    │  └─────────┴─────────────┘  │  │
│  └─────────────────────┘    └─────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
┌─────────────────────────────────────────────────────────────┐
│                    应用层 (Application)                      │
│  ┌─────────────────────┐    ┌─────────────────────────────┐  │
│  │   Event Handler     │    │      State Manager          │  │
│  │   Command Router    │    │   ┌─────────┬─────────────┐  │  │
│  │                     │    │   │ App     │   UI        │  │  │
│  │                     │    │   │ State   │   State     │  │  │
│  │                     │    │   ├─────────┼─────────────┤  │  │
│  │                     │    │   │ Git     │  Agent      │  │  │
│  │                     │    │   │ State   │  State      │  │  │
│  │                     │    │   └─────────┴─────────────┘  │  │
│  └─────────────────────┘    └─────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
┌─────────────────────────────────────────────────────────────┐
│                     业务层 (Domain)                          │
│  ┌─────────────────────────────────────────────────────────┐  │
│  │              Git工作流管理 (Git Workflow)              │  │
│  │  ┌─────────────┬─────────────┬─────────────────────────┐  │  │
│  │  │   Branch    │     Tag     │     Remote & GitFlow   │  │  │
│  │  │   Manager   │   Manager   │       Manager           │  │  │
│  │  └─────────────┴─────────────┴─────────────────────────┘  │  │
│  └─────────────────────────────────────────────────────────┘  │
│  ┌─────────────────────────────────────────────────────────┐  │
│  │                Agent系统 (Agent System)                │  │
│  │  ┌─────────────┬─────────────┬─────────────────────────┐  │  │
│  │  │   Agent     │  Message    │     Agent Pool &        │  │  │
│  │  │  Manager    │    Bus      │    Task Scheduler       │  │  │
│  │  └─────────────┴─────────────┴─────────────────────────┘  │  │
│  │  ┌─────────────┬─────────────┬─────────────────────────┐  │  │
│  │  │   Commit    │    Code     │   Review & Search       │  │  │
│  │  │   Agent     │  Analysis   │      Agents             │  │  │
│  │  └─────────────┴─────────────┴─────────────────────────┘  │  │
│  └─────────────────────────────────────────────────────────┘  │
│  ┌─────────────────────────────────────────────────────────┐  │
│  │               MCP协议层 (MCP Protocol)                   │  │
│  │  ┌─────────────┬─────────────┬─────────────────────────┐  │  │
│  │  │    MCP      │    MCP      │     Resource &          │  │  │
│  │  │   Server    │   Client    │    Tool Registry        │  │  │
│  │  └─────────────┴─────────────┴─────────────────────────┘  │  │
│  │  ┌─────────────┬─────────────┬─────────────────────────┐  │  │
│  │  │  WebSocket  │    HTTP     │      Standard I/O       │  │  │
│  │  │  Transport  │  Transport  │      Transport          │  │  │
│  │  └─────────────┴─────────────┴─────────────────────────┘  │  │
│  └─────────────────────────────────────────────────────────┘  │
│  ┌─────────────────────────────────────────────────────────┐  │
│  │               核心服务 (Core Services)                   │  │
│  │  ┌─────────────┬─────────────┬─────────────────────────┐  │  │
│  │  │    Git      │     AI      │       Search            │  │  │
│  │  │ Operations  │  Services   │       Engine            │  │  │
│  │  └─────────────┴─────────────┴─────────────────────────┘  │  │
│  └─────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
┌─────────────────────────────────────────────────────────────┐
│                 基础设施层 (Infrastructure)                   │
│  ┌─────────────────────────────────────────────────────────┐  │
│  │                网络与通信 (Network & Communication)      │  │
│  │  ┌─────────────┬─────────────┬─────────────────────────┐  │  │
│  │  │   HTTP      │  WebSocket  │       Terminal          │  │  │
│  │  │   Client    │   Client    │       I/O               │  │  │
│  │  └─────────────┴─────────────┴─────────────────────────┘  │  │
│  └─────────────────────────────────────────────────────────┘  │
│  ┌─────────────────────────────────────────────────────────┐  │
│  │               存储与配置 (Storage & Configuration)        │  │
│  │  ┌─────────────┬─────────────┬─────────────────────────┐  │  │
│  │  │    File     │   Config    │       Cache &           │  │  │
│  │  │   System    │  Manager    │     Logging             │  │  │
│  │  └─────────────┴─────────────┴─────────────────────────┘  │  │
│  └─────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

### 1.2 模块依赖关系
```
main.rs
├── app/
│   ├── state.rs           (应用状态管理)
│   ├── events.rs          (事件处理系统)
│   └── config.rs          (配置管理)
├── ui/
│   ├── components/        (UI组件)
│   ├── layout.rs          (布局管理)
│   └── themes.rs          (主题系统)
├── git/
│   ├── repository.rs      (仓库操作)
│   ├── diff.rs           (差异处理)
│   └── status.rs         (状态检查)
├── ai/
│   ├── client.rs         (AI服务客户端)
│   ├── prompt.rs         (提示词管理)
│   └── suggestions.rs    (建议处理)
├── search/
│   ├── engine.rs         (搜索引擎)
│   ├── indexer.rs        (索引管理)
│   └── filters.rs        (过滤器)
└── utils/
    ├── logger.rs         (日志系统)
    ├── cache.rs          (缓存管理)
    └── async_task.rs     (异步任务)
```

### 1.3 系统交互架构图

#### 1.3.1 整体系统交互流程
```
                          ┌─────────────────────────────────────┐
                          │              用户界面               │
                          │        (Terminal UI - TUI)        │
                          │                                   │
                          │  ┌───────┐ ┌───────┐ ┌────────┐  │
                          │  │ File  │ │ Diff  │ │Branch  │  │
                          │  │ List  │ │Viewer │ │Manager │  │
                          │  └───────┘ └───────┘ └────────┘  │
                          └─────────────┬───────────────────────┘
                                      │ Events & Commands
                                      ▼
          ┌─────────────────────────────────────────────────────────────┐
          │                    事件处理与路由                            │
          │                  (Event Handler)                          │
          │                                                           │
          │  ┌──────────────┐  ┌──────────────┐  ┌──────────────────┐  │
          │  │  Keyboard    │  │    Git       │  │     Agent        │  │
          │  │  Events      │  │  Commands    │  │   Commands       │  │
          │  └──────────────┘  └──────────────┘  └──────────────────┘  │
          └─────┬─────────────────┬─────────────────┬─────────────────────┘
                │                 │                 │
                ▼                 ▼                 ▼
┌─────────────────────┐ ┌─────────────────────┐ ┌─────────────────────┐
│    Git工作流管理     │ │    Agent系统管理     │ │   MCP协议处理        │
│   (Git Workflow)   │ │  (Agent System)    │ │  (MCP Protocol)    │
│                    │ │                    │ │                    │
│ ┌─────────────────┐ │ │ ┌─────────────────┐ │ │ ┌─────────────────┐ │
│ │  Branch Manager │ │ │ │  Agent Manager  │ │ │ │  MCP Server     │ │
│ │  Tag Manager    │ │ │ │  Message Bus    │ │ │ │  MCP Client     │ │
│ │  Remote Manager │ │ │ │  Task Scheduler │ │ │ │  Transport Mgr  │ │
│ └─────────────────┘ │ │ └─────────────────┘ │ │ └─────────────────┘ │
│         │           │ │         │           │ │         │           │
│         ▼           │ │         ▼           │ │         ▼           │
│ ┌─────────────────┐ │ │ ┌─────────────────┐ │ │ ┌─────────────────┐ │
│ │   Git Core      │ │ │ │  Commit Agent   │ │ │ │   Resources     │ │
│ │  Operations     │ │ │ │  Analysis Agent │ │ │ │   Tools         │ │ │
│ │                 │ │ │ │  Review Agent   │ │ │ │   Handlers      │ │
│ └─────────────────┘ │ │ └─────────────────┘ │ │ └─────────────────┘ │
└─────────────────────┘ └─────────────────────┘ └─────────────────────┘
          │                         │                         │
          ▼                         ▼                         ▼
          ┌─────────────────────────────────────────────────────────────┐
          │                      基础服务层                              │
          │                  (Infrastructure)                         │
          │                                                           │
          │ ┌──────────────┐ ┌──────────────┐ ┌──────────────────────┐ │
          │ │   Git2       │ │   HTTP       │ │      File System     │ │
          │ │  (libgit2)   │ │  Client      │ │                      │ │
          │ └──────────────┘ └──────────────┘ └──────────────────────┘ │
          │                                                           │
          │ ┌──────────────┐ ┌──────────────┐ ┌──────────────────────┐ │
          │ │  WebSocket   │ │   Terminal   │ │      Config &        │ │
          │ │   Client     │ │     I/O      │ │       Cache          │ │
          │ └──────────────┘ └──────────────┘ └──────────────────────┘ │
          └─────────────────────────────────────────────────────────────┘
```

#### 1.3.2 Agent系统内部交互架构
```
                    ┌─────────────────────────────────────┐
                    │            Agent管理器               │
                    │         (Agent Manager)            │
                    │                                   │
                    │  ┌─────────────────────────────┐   │
                    │  │      Agent Registry         │   │
                    │  │   (Agent注册和发现)           │   │
                    │  └─────────────────────────────┘   │
                    └─────────────┬───────────────────────┘
                                │ Agent Lifecycle Management
                                ▼
        ┌─────────────────────────────────────────────────────────┐
        │                    消息总线                              │
        │                 (Message Bus)                          │
        │                                                       │
        │  ┌──────────────┐ ┌──────────────┐ ┌────────────────┐ │
        │  │   Task       │ │   Message    │ │    Result      │ │
        │  │  Routing     │ │   Queue      │ │   Collector    │ │
        │  └──────────────┘ └──────────────┘ └────────────────┘ │
        └─────┬─────────────────┬─────────────────┬─────────────┘
              │                 │                 │
              ▼                 ▼                 ▼
    ┌─────────────────┐ ┌─────────────────┐ ┌─────────────────┐
    │   Commit Agent  │ │ Code Analysis   │ │   Review Agent  │
    │                 │ │     Agent       │ │                 │
    │ ┌─────────────┐ │ │ ┌─────────────┐ │ │ ┌─────────────┐ │
    │ │AI Client    │ │ │ │Rule Engine  │ │ │ │Quality Check│ │
    │ │Template Mgr │ │ │ │Language Sup │ │ │ │Pattern Det  │ │
    │ │Cache System │ │ │ │AST Parser   │ │ │ │Refactor Sug │ │
    │ └─────────────┘ │ │ └─────────────┘ │ │ └─────────────┘ │
    └─────────────────┘ └─────────────────┘ └─────────────────┘
              │                 │                 │
              ▼                 ▼                 ▼
         [AI Service]      [Code Analysis]    [Review Results]
         [Suggestions]      [Metrics & Issues]  [Recommendations]
```

#### 1.3.3 MCP协议通信架构
```
    ┌─────────────────────────────────────────────────────────────┐
    │                      AI-Commit TUI                          │
    │                    (MCP Client)                            │
    │                                                           │
    │  ┌──────────────┐  ┌──────────────┐  ┌─────────────────┐  │
    │  │   Resource   │  │     Tool     │  │   Connection    │  │
    │  │   Manager    │  │   Registry   │  │    Manager      │  │
    │  └──────────────┘  └──────────────┘  └─────────────────┘  │
    └─────────────┬─────────────────┬─────────────────┬─────────┘
                  │                 │                 │
          ┌───────▼──────┐ ┌────────▼─────────┐ ┌─────▼─────────┐
          │  WebSocket   │ │      HTTP        │ │   Stdio       │
          │  Transport   │ │   Transport      │ │  Transport    │
          └───────┬──────┘ └────────┬─────────┘ └─────┬─────────┘
                  │                 │                 │
                  └─────────┬───────┘                 │
                            │                         │
                    ┌───────▼─────────────────────────▼────────┐
                    │            JSON-RPC 2.0                 │
                    │          Message Protocol                │
                    │                                         │
                    │ ┌─────────────┐ ┌─────────────────────┐ │
                    │ │   Request   │ │      Response       │ │
                    │ │  Messages   │ │     Messages        │ │
                    │ └─────────────┘ └─────────────────────┘ │
                    └───────────┬───────────┬─────────────────┘
                                │           │
                        ┌───────▼───────────▼──────┐
                        │      MCP Server          │
                        │                          │
                        │ ┌──────────────────────┐ │
                        │ │   Resource Server    │ │
                        │ │                      │ │
                        │ │ ┌─────────────────┐  │ │
                        │ │ │   Git Tools     │  │ │
                        │ │ │   File Tools    │  │ │
                        │ │ │   Search Tools  │  │ │
                        │ │ │   AI Tools      │  │ │
                        │ │ └─────────────────┘  │ │
                        │ └──────────────────────┘ │
                        │                          │
                        │ ┌──────────────────────┐ │
                        │ │    Tool Server       │ │
                        │ │                      │ │
                        │ │ ┌─────────────────┐  │ │
                        │ │ │Git Repository   │  │ │
                        │ │ │File System     │  │ │
                        │ │ │Code Analysis    │  │ │
                        │ │ │Documentation    │  │ │
                        │ │ └─────────────────┘  │ │
                        │ └──────────────────────┘ │
                        └──────────────────────────┘
```

## 2. 核心模块架构

### 2.1 应用状态模块 (app/)
```rust
// app/state.rs 架构设计
pub struct AppState {
    // UI状态
    current_tab: TabState,
    selected_file: Option<FileItem>,
    modal_stack: Vec<Modal>,

    // Git状态
    repository: GitRepository,
    file_status: Vec<FileStatus>,
    commit_history: Vec<CommitInfo>,

    // AI状态
    ai_suggestions: Vec<AISuggestion>,
    current_suggestion: Option<usize>,

    // 搜索状态
    search_query: String,
    search_results: SearchResults,
    active_filters: Vec<Filter>,

    // 配置状态
    config: AppConfig,
    keybindings: KeyBindings,
}

// 状态管理策略
pub trait StateManager {
    fn update_state(&mut self, action: Action) -> Result<()>;
    fn get_state(&self) -> &AppState;
    fn subscribe_to_changes(&mut self, callback: StateChangeCallback);
}
```

### 2.2 事件系统模块 (app/events.rs)
```rust
// 事件类型定义
pub enum AppEvent {
    // 键盘事件
    KeyPressed(KeyEvent),

    // Git事件
    GitStatusChanged,
    FileStaged(String),
    FileUnstaged(String),
    CommitCreated(String),

    // AI事件
    AISuggestionReceived(Vec<AISuggestion>),
    AISuggestionSelected(usize),

    // UI事件
    TabChanged(TabType),
    ModalOpened(Modal),
    ModalClosed,

    // 搜索事件
    SearchQueryChanged(String),
    FilterApplied(Filter),

    // 系统事件
    Tick,
    Quit,
    Error(AppError),
}

// 事件处理器架构
pub struct EventHandler {
    state: Arc<Mutex<AppState>>,
    git_service: Arc<GitService>,
    ai_service: Arc<AIService>,
    search_service: Arc<SearchService>,
}

impl EventHandler {
    pub async fn handle_event(&mut self, event: AppEvent) -> Result<()>;
}
```

### 2.3 UI组件模块 (ui/)

#### 2.3.1 UI组件层次架构图
```
                    ┌─────────────────────────────────────────────────────────┐
                    │                   应用窗口                                │
                    │               (Application Window)                      │
                    │                                                       │
                    │ ┌─────────────────────────────────────────────────────┐ │
                    │ │              Header Navigation                       │ │
                    │ │         (Navigation Tab Component)                  │ │
                    │ │                                                   │ │
                    │ │ ┌──────┐┌──────┐┌──────┐┌──────┐┌──────┐              │ │
                    │ │ │Branches││Tags ││Stash ││Status││Remotes│              │ │
                    │ │ └──────┘└──────┘└──────┘└──────┘└──────┘              │ │
                    │ │ ┌──────────┐                                    │ │
                    │ │ │Git工作流  │                                    │ │
                    │ │ └──────────┘                                    │ │
                    │ └─────────────────────────────────────────────────────┘ │
                    │                                                       │
    ┌───────────────┼─────────────────────────────────────────────────────────┼───────────────┐
    │               │              Main Content Area                          │               │
    │               │                                                       │               │
    │ ┌─────────────┐ │ ┌─────────────────────────────────────────────────────┐ │ ┌─────────────┐ │
    │ │   Sidebar   │ │ │           Dynamic Content Area                      │ │ │   Detail    │ │
    │ │ Component   │ │ │          (基于选中的Tab变化)                          │ │ │   Panel     │ │
    │ │             │ │ │                                                   │ │ │ (Optional)  │ │
    │ │┌───────────┐│ │ │ ┌─────────────────────────────────────────────────┐ │ │ │             │ │
    │ ││Repository ││ │ │ │          Current View Content                   │ │ │ │┌───────────┐│ │
    │ ││Info       ││ │ │ │                                                 │ │ │ ││  Diff     ││ │
    │ ││           ││ │ │ │ Branches Tab: 分支列表 + 分支图                    │ │ │ ││ Viewer    ││ │
    │ ││● 仓库名称  ││ │ │ │ Tags Tab: 标签列表 + 版本管理                       │ │ │ ││           ││ │
    │ ││● 当前分支  ││ │ │ │ Stash Tab: 暂存列表 + 操作按钮                     │ │ │ │└───────────┘│ │
    │ ││● 提交数量  ││ │ │ │ Status Tab: 文件状态 + 差异显示                    │ │ │ │             │ │
    │ ││● 分支数量  ││ │ │ │ Remotes Tab: 远程仓库 + 同步状态                   │ │ │ │┌───────────┐│ │
    │ ││● 标签数量  ││ │ │ │ Git工作流 Tab: 工作流状态 + 操作面板                │ │ │ ││  Agent    ││ │
    │ ││● Stash数量││ │ │ │                                                 │ │ │ ││Suggestions││ │
    │ │└───────────┘│ │ │ │                                                 │ │ │ ││           ││ │
    │ │             │ │ │ │                                                 │ │ │ │└───────────┘│ │
    │ │┌───────────┐│ │ │ └─────────────────────────────────────────────────┘ │ │ │             │ │
    │ ││Quick      ││ │ │                                                   │ │ │┌───────────┐│ │
    │ ││Actions    ││ │ │ ┌─────────────────────────────────────────────────┐ │ │ ││  File     ││ │
    │ ││           ││ │ │ │         Contextual Actions Bar                  │ │ │ ││ Explorer  ││ │
    │ ││● Commit   ││ │ │ │     (基于当前Tab显示相关操作按钮)                 │ │ │ ││           ││ │
    │ ││● Pull     ││ │ │ │                                                 │ │ │ │└───────────┘│ │
    │ ││● Push     ││ │ │ └─────────────────────────────────────────────────┘ │ │ └─────────────┘ │
    │ ││● Refresh  ││ │ │                                                   │ │                 │
    │ │└───────────┘│ │ └─────────────────────────────────────────────────────┘ │                 │
    │ └─────────────┘ └─────────────────────────────────────────────────────────┘                 │
    │               ┌─────────────────────────────────────────────────────────┐                   │
    │               │              Status Bar Component                       │                   │
    │               │                                                       │                   │
    │               │ ┌────────────┐  ┌──────────────┐  ┌─────────────────┐  │                   │
    │               │ │Git Status  │  │ AI Agent     │  │ Network Status  │  │                   │
    │               │ │Info        │  │ Status       │  │ & Performance   │  │                   │
    │               │ └────────────┘  └──────────────┘  └─────────────────┘  │                   │
    │               └─────────────────────────────────────────────────────────┘                   │
    └─────────────────────────────────────────────────────────────────────────────────────────────┘
```

#### 2.3.2 UI组件交互流程图
```
                    用户输入 (Keyboard/Mouse Events)
                                    │
                                    ▼
                        ┌─────────────────────────┐
                        │      Event Router       │
                        │   (事件路由和分发器)       │
                        └─────────────┬───────────┘
                                    │
                    ┌───────────────┼───────────────┐
                    ▼               ▼               ▼
            ┌─────────────────┐ ┌─────────────────┐ ┌─────────────────┐
            │   Focus-based   │ │   Global        │ │   Modal         │
            │   Event Handler │ │   Shortcuts     │ │   Event Handler │ │
            └─────────────────┘ └─────────────────┘ └─────────────────┘
                    │               │               │
                    ▼               ▼               ▼
            ┌─────────────────┐ ┌─────────────────┐ ┌─────────────────┐
            │   Component     │ │   Application   │ │   Modal Stack   │
            │   State Update  │ │   State Update  │ │   Management    │
            └─────────────────┘ └─────────────────┘ └─────────────────┘
                    │               │               │
                    └───────────────┼───────────────┘
                                    ▼
                        ┌─────────────────────────┐
                        │    State Synchronizer   │
                        │   (状态同步和通知)        │
                        └─────────────┬───────────┘
                                    │
                    ┌───────────────┼───────────────┐
                    ▼               ▼               ▼
            ┌─────────────────┐ ┌─────────────────┐ ┌─────────────────┐
            │   UI Component  │ │   Layout        │ │   Theme         │
            │   Re-render     │ │   Recalculation │ │   Update        │
            └─────────────────┘ └─────────────────┘ └─────────────────┘
                    │               │               │
                    └───────────────┼───────────────┘
                                    ▼
                        ┌─────────────────────────┐
                        │    Terminal Renderer    │
                        │   (终端画面渲染)         │
                        └─────────────────────────┘
```

#### 2.3.3 响应式布局系统架构
```
                    ┌─────────────────────────────────────┐
                    │         Layout Engine               │
                    │      (响应式布局引擎)                │
                    │                                   │
                    │  ┌─────────────────────────────┐   │
                    │  │    Terminal Size Detector   │   │
                    │  │     (终端尺寸检测器)          │   │
                    │  └─────────────┬───────────────┘   │
                    └─────────────────┼─────────────────────┘
                                    │ Size Change Events
                                    ▼
        ┌─────────────────────────────────────────────────────────────┐
        │                  Breakpoint Manager                         │
        │                 (断点管理器)                                  │
        │                                                           │
        │  ┌──────────────┐ ┌──────────────┐ ┌────────────────────┐ │
        │  │   Mobile     │ │   Tablet     │ │      Desktop       │ │
        │  │  (< 80 cols) │ │ (80-120 cols)│ │     (> 120 cols)   │ │
        │  └──────────────┘ └──────────────┘ └────────────────────┘ │
        └─────┬─────────────────┬─────────────────┬─────────────────┘
              │                 │                 │
              ▼                 ▼                 ▼
    ┌─────────────────┐ ┌─────────────────┐ ┌─────────────────┐
    │  Compact Layout │ │  Standard Layout│ │  Extended Layout│
    │                 │ │                 │ │                 │
    │ ┌─────────────┐ │ │ ┌─────────────┐ │ │ ┌─────────────┐ │
    │ │Single Column│ │ │ │Two Columns  │ │ │ │Three Columns│ │
    │ │   Stacked   │ │ │ │ Side + Main │ │ │ │Side+Main+Det│ │
    │ └─────────────┘ │ │ └─────────────┘ │ │ └─────────────┘ │
    │                 │ │                 │ │                 │
    │ ┌─────────────┐ │ │ ┌─────────────┐ │ │ ┌─────────────┐ │
    │ │   Tabs      │ │ │ │ Collapsible │ │ │ │   Always    │ │
    │ │  Navigator  │ │ │ │   Sidebar   │ │ │ │   Visible   │ │
    │ └─────────────┘ │ │ └─────────────┘ │ │ └─────────────┘ │
    └─────────────────┘ └─────────────────┘ └─────────────────┘
```

#### 2.3.4 主题系统架构
```
                        ┌─────────────────────────┐
                        │      Theme Manager      │
                        │      (主题管理器)        │
                        │                       │
                        │ ┌─────────────────────┐ │
                        │ │   Theme Registry    │ │
                        │ │   (主题注册表)       │ │
                        │ └─────────────────────┘ │
                        └─────────────┬───────────┘
                                    │
                ┌───────────────────┼───────────────────┐
                ▼                   ▼                   ▼
        ┌─────────────────┐ ┌─────────────────┐ ┌─────────────────┐
        │   Built-in      │ │    User         │ │    Dynamic      │
        │   Themes        │ │   Themes        │ │    Themes       │
        │                 │ │                 │ │                 │
        │ ┌─────────────┐ │ │ ┌─────────────┐ │ │ ┌─────────────┐ │
        │ │Dark Theme   │ │ │ │Custom Dark  │ │ │ │Auto Theme   │ │
        │ │Light Theme  │ │ │ │Custom Light │ │ │ │Time-based   │ │
        │ │High Contrast│ │ │ │Terminal     │ │ │ │System-based │ │
        │ └─────────────┘ │ │ └─────────────┘ │ │ └─────────────┘ │
        └─────────────────┘ └─────────────────┘ └─────────────────┘
                │                   │                   │
                └───────────────────┼───────────────────┘
                                    ▼
                        ┌─────────────────────────┐
                        │    Color Processor      │
                        │    (颜色处理器)          │
                        │                       │
                        │ ┌─────────────────────┐ │
                        │ │  Syntax Highlighting│ │
                        │ │  Status Colors      │ │
                        │ │  Border Colors      │ │
                        │ │  Background Colors  │ │
                        │ └─────────────────────┘ │
                        └─────────────────────────┘
```

```rust
// ui/components/ 组件架构 (基于tui-demo.html实际布局)
pub trait Component {
    fn render(&self, frame: &mut Frame, area: Rect, state: &AppState);
    fn handle_event(&mut self, event: &AppEvent) -> Option<Action>;
    fn update(&mut self, state: &AppState);
}

// 主要组件结构 (与demo布局保持一致)
pub struct HeaderNavigationComponent {
    active_tab: TabType,
    tabs: Vec<NavigationTab>,
    tab_renderer: TabRenderer,
}

pub enum TabType {
    Branches,
    Tags,
    Stash,
    Status,
    Remotes,
    GitWorkflow,    // Git工作流
}

pub struct NavigationTab {
    name: String,
    tab_type: TabType,
    is_active: bool,
    badge_count: Option<u32>, // 用于显示计数徽章
}

pub struct SidebarComponent {
    repo_info: RepositoryInfoPanel,
    quick_actions: QuickActionsPanel,
}

pub struct RepositoryInfoPanel {
    repo_name: String,
    current_branch: String,
    commits_count: u32,
    branches_count: u32,
    tags_count: u32,
    stashes_count: u32,
    info_renderer: InfoRenderer,
}

pub struct QuickActionsPanel {
    actions: Vec<QuickAction>,
    selected_action: Option<usize>,
}

pub struct QuickAction {
    name: String,
    action_type: ActionType,
    enabled: bool,
    icon: Option<String>,
}

pub enum ActionType {
    Commit,
    Pull,
    Push,
    Refresh,
    Stash,
    Merge,
}

// 动态内容区域 (基于选中的Tab变化)
pub struct DynamicContentArea {
    current_view: TabType,
    view_components: HashMap<TabType, Box<dyn ViewComponent>>,
    contextual_actions: ContextualActionsBar,
}

pub trait ViewComponent: Component {
    fn tab_type(&self) -> TabType;
    fn get_contextual_actions(&self) -> Vec<ContextualAction>;
}

// 具体的Tab视图组件
pub struct BranchesView {
    branch_list: BranchList,
    branch_graph: BranchGraph,
    branch_operations: BranchOperationsPanel,
}

pub struct TagsView {
    tag_list: TagList,
    version_timeline: VersionTimeline,
    tag_operations: TagOperationsPanel,
}

pub struct StashView {
    stash_list: StashList,
    stash_preview: StashPreviewPanel,
    stash_operations: StashOperationsPanel,
}

pub struct StatusView {
    file_list: FileStatusList,
    diff_viewer: DiffViewer,
    staging_area: StagingAreaPanel,
}

pub struct RemotesView {
    remote_list: RemoteList,
    sync_status: SyncStatusPanel,
    remote_operations: RemoteOperationsPanel,
}

pub struct GitWorkflowView {
    workflow_status: WorkflowStatusPanel,
    flow_diagram: GitFlowDiagram,
    workflow_operations: WorkflowOperationsPanel,
}

// 上下文操作栏
pub struct ContextualActionsBar {
    actions: Vec<ContextualAction>,
    current_tab: TabType,
}

pub struct ContextualAction {
    name: String,
    action: Box<dyn Fn() -> Action>,
    enabled: bool,
    shortcut: Option<String>,
}

// 可选的详情面板
pub struct DetailPanel {
    is_visible: bool,
    content: DetailContent,
}

pub enum DetailContent {
    DiffViewer(DiffViewer),
    AgentSuggestions(AgentSuggestionsPanel),
    FileExplorer(FileExplorerPanel),
    MCPResources(MCPResourcePanel),
}

// 状态栏组件
pub struct StatusBarComponent {
    git_status_info: GitStatusInfo,
    agent_status: AgentStatusIndicator,
    network_status: NetworkStatusIndicator,
    performance_info: PerformanceInfo,
}

// 响应式布局管理 (适配demo的实际布局)
pub struct LayoutManager {
    header_nav_height: u16,
    sidebar_width: u16,
    detail_panel_width: u16,
    statusbar_height: u16,
    responsive_breakpoints: Vec<Breakpoint>,
    current_layout: LayoutMode,
}

// Agent系统UI组件 (集成到详情面板)
pub struct AgentSuggestionsPanel {
    suggestions: Vec<AgentSuggestion>,
    selected_suggestion: Option<usize>,
    suggestion_renderer: SuggestionRenderer,
    confidence_indicators: Vec<ConfidenceBar>,
}

// MCP协议UI组件 (集成到详情面板)
pub struct MCPResourcePanel {
    mcp_connections: Vec<MCPConnection>,
    resource_browser: MCPResourceBrowser,
    tool_explorer: MCPToolExplorer,
    connection_status: ConnectionStatusWidget,
}
```

### 2.4 Git服务模块 (git/)
```rust
// git/repository.rs 架构
pub struct GitService {
    repo: Repository,
    config: GitConfig,
    cache: Arc<RwLock<GitCache>>,
}

impl GitService {
    // 仓库基础操作
    pub fn open_repository(path: &Path) -> Result<Self>;
    pub fn get_status(&self) -> Result<Vec<FileStatus>>;
    pub fn get_diff(&self, file: &str) -> Result<Diff>;
    pub fn stage_file(&self, file: &str) -> Result<()>;
    pub fn unstage_file(&self, file: &str) -> Result<()>;
    pub fn commit(&self, message: &str) -> Result<Oid>;

    // 高级操作
    pub fn get_commit_history(&self, limit: usize) -> Result<Vec<CommitInfo>>;
    pub fn get_branch_info(&self) -> Result<BranchInfo>;
    pub fn get_remote_info(&self) -> Result<Vec<RemoteInfo>>;
}

// Git工作流管理架构
pub struct GitWorkflowManager {
    git_service: Arc<GitService>,
    workflow_config: WorkflowConfig,
    branch_manager: BranchManager,
    tag_manager: TagManager,
    remote_manager: RemoteManager,
}

impl GitWorkflowManager {
    // 分支管理
    pub fn create_branch(&self, name: &str, from: Option<&str>) -> Result<()>;
    pub fn switch_branch(&self, name: &str) -> Result<()>;
    pub fn delete_branch(&self, name: &str, force: bool) -> Result<()>;
    pub fn merge_branch(&self, from: &str, to: &str, strategy: MergeStrategy) -> Result<()>;
    pub fn list_branches(&self, filter: BranchFilter) -> Result<Vec<BranchInfo>>;

    // 标签管理
    pub fn create_tag(&self, name: &str, message: Option<&str>, target: Option<&str>) -> Result<()>;
    pub fn delete_tag(&self, name: &str) -> Result<()>;
    pub fn list_tags(&self) -> Result<Vec<TagInfo>>;
    pub fn push_tag(&self, name: &str, remote: &str) -> Result<()>;

    // 远程仓库管理
    pub fn add_remote(&self, name: &str, url: &str) -> Result<()>;
    pub fn remove_remote(&self, name: &str) -> Result<()>;
    pub fn fetch(&self, remote: Option<&str>) -> Result<FetchResult>;
    pub fn pull(&self, remote: &str, branch: &str, strategy: PullStrategy) -> Result<()>;
    pub fn push(&self, remote: &str, branch: &str, force: bool) -> Result<()>;

    // Git Flow工作流
    pub fn init_gitflow(&self, config: GitFlowConfig) -> Result<()>;
    pub fn start_feature(&self, name: &str) -> Result<()>;
    pub fn finish_feature(&self, name: &str) -> Result<()>;
    pub fn start_hotfix(&self, name: &str, version: &str) -> Result<()>;
    pub fn finish_hotfix(&self, name: &str) -> Result<()>;
    pub fn start_release(&self, version: &str) -> Result<()>;
    pub fn finish_release(&self, version: &str) -> Result<()>;

    // 高级Git操作
    pub fn interactive_rebase(&self, base: &str) -> Result<RebaseSession>;
    pub fn cherry_pick(&self, commit: &str) -> Result<()>;
    pub fn stash_save(&self, message: Option<&str>, include_untracked: bool) -> Result<()>;
    pub fn stash_pop(&self, stash_index: Option<usize>) -> Result<()>;
    pub fn stash_list(&self) -> Result<Vec<StashInfo>>;
}

// 分支管理器
pub struct BranchManager {
    git_service: Arc<GitService>,
    protection_rules: Vec<BranchProtectionRule>,
}

impl BranchManager {
    pub fn get_branch_graph(&self) -> Result<BranchGraph>;
    pub fn check_protection_rules(&self, branch: &str, operation: BranchOperation) -> Result<()>;
    pub fn get_branch_status(&self, branch: &str) -> Result<BranchStatus>;
    pub fn compare_branches(&self, base: &str, compare: &str) -> Result<BranchComparison>;
}

// 标签管理器
pub struct TagManager {
    git_service: Arc<GitService>,
    semver_config: SemVerConfig,
}

impl TagManager {
    pub fn suggest_next_version(&self, change_type: ChangeType) -> Result<String>;
    pub fn validate_tag_name(&self, name: &str) -> Result<()>;
    pub fn get_tag_changelog(&self, from: &str, to: &str) -> Result<String>;
}

// 远程管理器
pub struct RemoteManager {
    git_service: Arc<GitService>,
    auth_manager: AuthManager,
}

impl RemoteManager {
    pub fn sync_with_remote(&self, remote: &str) -> Result<SyncStatus>;
    pub fn resolve_conflicts(&self, strategy: ConflictResolutionStrategy) -> Result<()>;
    pub fn get_remote_status(&self, remote: &str) -> Result<RemoteStatus>;
}

// 差异处理架构
pub struct DiffProcessor {
    syntax_highlighter: SyntaxHighlighter,
    diff_algorithm: DiffAlgorithm,
}

impl DiffProcessor {
    pub fn process_diff(&self, diff: &Diff) -> Result<ProcessedDiff>;
    pub fn highlight_syntax(&self, content: &str, language: &str) -> String;
    pub fn calculate_stats(&self, diff: &Diff) -> DiffStats;
}
```

### 2.5 AI服务模块 (ai/) - Agent架构
```rust
// ai/core/agent_manager.rs 架构
pub struct AgentManager {
    agents: HashMap<String, Box<dyn Agent>>,
    message_bus: Arc<MessageBus>,
    task_scheduler: TaskScheduler,
    health_monitor: HealthMonitor,
    config: AgentConfig,
}

impl AgentManager {
    pub async fn register_agent(&mut self, agent: Box<dyn Agent>) -> Result<()>;
    pub async fn dispatch_task(&self, task: AgentTask) -> Result<AgentResult>;
    pub async fn get_agent_status(&self, name: &str) -> Option<AgentStatus>;
    pub async fn shutdown_all(&mut self) -> Result<()>;
}

// ai/agents/base_agent.rs 基础Agent trait
pub trait Agent: Send + Sync {
    fn name(&self) -> &str;
    fn capabilities(&self) -> Vec<AgentCapability>;
    async fn handle_task(&mut self, task: AgentTask) -> Result<AgentResult>;
    fn health_check(&self) -> HealthStatus;
    async fn shutdown(&mut self) -> Result<()>;
}

// ai/agents/commit_agent.rs 提交消息生成Agent
pub struct CommitAgent {
    ai_client: Arc<AIClient>,
    prompt_manager: PromptManager,
    cache: SuggestionCache,
}

impl Agent for CommitAgent {
    async fn handle_task(&mut self, task: AgentTask) -> Result<AgentResult> {
        match task {
            AgentTask::GenerateCommitMessage(changes) => {
                let suggestions = self.generate_suggestions(changes).await?;
                Ok(AgentResult::CommitSuggestions(suggestions))
            }
            _ => Err(AgentError::UnsupportedTask),
        }
    }
}

// ai/mcp/server.rs MCP服务器架构
pub struct MCPServer {
    transport: Box<dyn MCPTransport>,
    resources: ResourceRegistry,
    tools: ToolRegistry,
    handlers: HandlerRegistry,
    clients: ClientManager,
    config: MCPConfig,
}

impl MCPServer {
    pub async fn start(&mut self) -> Result<()>;
    pub async fn register_resource(&mut self, resource: Resource) -> Result<()>;
    pub async fn register_tool(&mut self, tool: Tool) -> Result<()>;
    pub async fn handle_request(&self, request: MCPRequest) -> MCPResponse;
    pub async fn broadcast_notification(&self, notification: MCPNotification);
}

// ai/mcp/client.rs MCP客户端架构
pub struct MCPClient {
    transport: Box<dyn MCPTransport>,
    server_capabilities: ServerCapabilities,
    resource_cache: ResourceCache,
    tool_registry: ToolRegistry,
}

impl MCPClient {
    pub async fn connect(&mut self, server_uri: &str) -> Result<()>;
    pub async fn list_resources(&self) -> Result<Vec<Resource>>;
    pub async fn call_tool(&self, name: &str, args: Value) -> Result<ToolResult>;
    pub async fn get_resource(&self, uri: &str) -> Result<ResourceContent>;
    pub async fn subscribe_to_resource(&self, uri: &str) -> Result<()>;
}
```

### 2.6 原有AI服务模块 (保留兼容)
```rust
// ai/client.rs 传统AI客户端架构
pub struct AIService {
    client: HttpClient,
    config: AIConfig,
    prompt_manager: PromptManager,
    cache: SuggestionCache,
}

impl AIService {
    pub async fn generate_commit_message(
        &self,
        changes: &[FileChange]
    ) -> Result<Vec<AISuggestion>>;

    pub async fn analyze_code_changes(
        &self,
        diff: &Diff
    ) -> Result<CodeAnalysis>;

    pub async fn suggest_improvements(
        &self,
        code: &str
    ) -> Result<Vec<Improvement>>;
}

// AI建议处理架构
pub struct SuggestionProcessor {
    ranking_algorithm: RankingAlgorithm,
    quality_scorer: QualityScorer,
    user_preferences: UserPreferences,
}

impl SuggestionProcessor {
    pub fn rank_suggestions(&self, suggestions: Vec<AISuggestion>) -> Vec<AISuggestion>;
    pub fn score_quality(&self, suggestion: &AISuggestion) -> f64;
    pub fn apply_user_preferences(&self, suggestions: &mut Vec<AISuggestion>);
}
```

### 2.7 搜索引擎模块 (search/)
```rust
// search/engine.rs 架构
pub struct SearchEngine {
    indexer: Indexer,
    query_parser: QueryParser,
    result_ranker: ResultRanker,
}

impl SearchEngine {
    pub fn search(&self, query: &str) -> Result<SearchResults>;
    pub fn search_files(&self, pattern: &str) -> Result<Vec<FileMatch>>;
    pub fn search_commits(&self, query: &str) -> Result<Vec<CommitMatch>>;
    pub fn search_content(&self, query: &str) -> Result<Vec<ContentMatch>>;
}

// 过滤系统架构
pub struct FilterEngine {
    filters: Vec<Box<dyn Filter>>,
    filter_chains: Vec<FilterChain>,
}

pub trait Filter {
    fn apply(&self, items: &[Item]) -> Vec<Item>;
    fn description(&self) -> &str;
}

// 预定义过滤器
pub struct StatusFilter;
pub struct DateRangeFilter;
pub struct AuthorFilter;
pub struct SizeFilter;
pub struct RegexFilter;
```

## 3. 异步架构设计

### 3.1 异步任务管理
```rust
// utils/async_task.rs 架构
pub struct TaskManager {
    executor: tokio::runtime::Runtime,
    task_queue: Arc<Mutex<VecDeque<Task>>>,
    active_tasks: Arc<Mutex<HashMap<TaskId, TaskHandle>>>,
    max_concurrent: usize,
}

pub enum Task {
    GitOperation(GitTask),
    AIRequest(AITask),
    SearchIndex(SearchTask),
    FileIO(IOTask),
}

impl TaskManager {
    pub async fn spawn_task(&self, task: Task) -> Result<TaskId>;
    pub async fn cancel_task(&self, id: TaskId) -> Result<()>;
    pub async fn wait_for_completion(&self, id: TaskId) -> Result<TaskResult>;
    pub fn get_task_progress(&self, id: TaskId) -> Option<Progress>;
}
```

### 3.2 消息传递架构
```rust
// 使用tokio channels进行模块间通信
pub struct MessageBus {
    // Git服务通道
    git_sender: mpsc::UnboundedSender<GitCommand>,
    git_receiver: mpsc::UnboundedReceiver<GitEvent>,

    // AI服务通道
    ai_sender: mpsc::UnboundedSender<AICommand>,
    ai_receiver: mpsc::UnboundedReceiver<AIEvent>,

    // UI更新通道
    ui_sender: mpsc::UnboundedSender<UIUpdate>,
    ui_receiver: mpsc::UnboundedReceiver<UIUpdate>,
}
```

## 4. 错误处理架构

### 4.1 错误类型系统
```rust
// 统一错误处理架构
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Git operation failed: {0}")]
    Git(#[from] GitError),

    #[error("AI service error: {0}")]
    AI(#[from] AIError),

    #[error("Search error: {0}")]
    Search(#[from] SearchError),

    #[error("IO error: {0}")]
    IO(#[from] std::io::Error),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("UI render error: {0}")]
    Render(String),
}

// 错误恢复策略
pub struct ErrorHandler {
    recovery_strategies: HashMap<ErrorType, RecoveryStrategy>,
    error_history: VecDeque<ErrorRecord>,
    notification_sender: mpsc::UnboundedSender<Notification>,
}
```

## 8. Agent和MCP架构详解

### 8.1 Agent系统架构
```rust
// Agent系统的核心架构
pub struct AgentSystem {
    manager: AgentManager,
    message_bus: Arc<MessageBus>,
    scheduler: TaskScheduler,
    monitor: HealthMonitor,
}

// Agent能力定义
#[derive(Debug, Clone)]
pub enum AgentCapability {
    CommitMessageGeneration,
    CodeAnalysis,
    CodeReview,
    SmartSearch,
    ContentGeneration,
    Custom(String),
}

// Agent任务类型
#[derive(Debug)]
pub enum AgentTask {
    GenerateCommitMessage(FileChanges),
    AnalyzeCode(CodeSnippet),
    ReviewPullRequest(PRData),
    SearchCode(SearchQuery),
    GenerateContent(ContentRequest),
}

// Agent结果类型
#[derive(Debug)]
pub enum AgentResult {
    CommitSuggestions(Vec<CommitSuggestion>),
    CodeAnalysis(AnalysisResult),
    ReviewComments(Vec<ReviewComment>),
    SearchResults(SearchResults),
    GeneratedContent(String),
}
```

### 8.2 MCP协议架构
```rust
// MCP协议的核心类型
#[derive(Debug, Clone)]
pub struct MCPRequest {
    pub id: RequestId,
    pub method: String,
    pub params: Option<Value>,
}

#[derive(Debug, Clone)]
pub struct MCPResponse {
    pub id: RequestId,
    pub result: Option<Value>,
    pub error: Option<MCPError>,
}

// MCP资源类型
#[derive(Debug, Clone)]
pub struct Resource {
    pub uri: String,
    pub name: String,
    pub description: Option<String>,
    pub mime_type: Option<String>,
}

// MCP工具类型
#[derive(Debug, Clone)]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
    pub handler: Arc<dyn ToolHandler>,
}

// MCP传输层trait
pub trait MCPTransport: Send + Sync {
    async fn send(&self, message: MCPMessage) -> Result<()>;
    async fn receive(&self) -> Result<MCPMessage>;
    async fn close(&self) -> Result<()>;
}
```

### 8.3 Agent与MCP集成架构
```rust
// Agent通过MCP暴露服务
pub struct AgentMCPBridge {
    agent_manager: Arc<AgentManager>,
    mcp_server: MCPServer,
    tool_registry: ToolRegistry,
}

impl AgentMCPBridge {
    // 将Agent能力注册为MCP工具
    pub async fn register_agent_tools(&mut self) -> Result<()> {
        let agents = self.agent_manager.list_agents().await;
        for agent in agents {
            let capabilities = agent.capabilities();
            for capability in capabilities {
                let tool = self.capability_to_tool(capability).await?;
                self.mcp_server.register_tool(tool).await?;
            }
        }
        Ok(())
    }

    // 处理MCP工具调用
    pub async fn handle_tool_call(&self, name: &str, args: Value) -> Result<ToolResult> {
        let task = self.args_to_agent_task(name, args)?;
        let agent_result = self.agent_manager.dispatch_task(task).await?;
        Ok(self.agent_result_to_tool_result(agent_result))
    }
}
```

## 9. 配置和插件架构

### 9.1 配置系统架构
```rust
// app/config.rs 配置架构
#[derive(Debug, Serialize, Deserialize)]
pub struct AppConfig {
    // UI配置
    pub ui: UIConfig,

    // Git配置
    pub git: GitConfig,

    // AI配置 (包含Agent和MCP)
    pub ai: AIConfig,

    // Agent配置
    pub agents: AgentConfig,

    // MCP配置
    pub mcp: MCPConfig,

    // 搜索配置
    pub search: SearchConfig,

    // 键盘快捷键
    pub keybindings: KeyBindings,
}

pub struct ConfigManager {
    config_path: PathBuf,
    config: AppConfig,
    watchers: Vec<notify::RecommendedWatcher>,
}

impl ConfigManager {
    pub fn load_config() -> Result<AppConfig>;
    pub fn save_config(&self, config: &AppConfig) -> Result<()>;
    pub fn watch_changes(&mut self) -> Result<mpsc::Receiver<ConfigEvent>>;
}
```

### 9.2 插件架构 (扩展Agent和MCP)
```rust
// 扩展插件系统以支持Agent和MCP
pub trait Plugin {
    fn name(&self) -> &str;
    fn version(&self) -> &str;
    fn init(&mut self, context: &PluginContext) -> Result<()>;
    fn handle_event(&mut self, event: &AppEvent) -> Result<Option<Action>>;
    fn shutdown(&mut self) -> Result<()>;

    // 新增Agent支持
    fn provides_agents(&self) -> Vec<Box<dyn Agent>> { vec![] }

    // 新增MCP支持
    fn provides_mcp_tools(&self) -> Vec<Tool> { vec![] }
    fn provides_mcp_resources(&self) -> Vec<Resource> { vec![] }
}

pub struct PluginManager {
    plugins: Vec<Box<dyn Plugin>>,
    plugin_config: HashMap<String, PluginConfig>,
    event_bus: Arc<EventBus>,

    // Agent集成
    agent_manager: Arc<AgentManager>,

    // MCP集成
    mcp_server: Arc<MCPServer>,
}
```

## 10. 性能优化架构

### 10.1 缓存系统架构 (包含Agent和MCP缓存)
```rust
// utils/cache.rs 缓存架构
pub struct CacheManager {
    // 内存缓存
    memory_cache: Arc<RwLock<LruCache<CacheKey, CacheValue>>>,

    // 磁盘缓存
    disk_cache: Option<DiskCache>,

    // 缓存策略
    strategies: HashMap<CacheType, CacheStrategy>,
}

pub enum CacheType {
    GitDiff,
    AISuggestions,
    SearchResults,
    SyntaxHighlighting,
    FileContent,
    AgentResults,      // 新增Agent结果缓存
    MCPResponses,      // 新增MCP响应缓存
    MCPResources,      // 新增MCP资源缓存
}

impl CacheManager {
    pub fn get<T>(&self, key: &CacheKey) -> Option<T>;
    pub fn set<T>(&self, key: CacheKey, value: T, ttl: Duration);
    pub fn invalidate(&self, pattern: &str);
    pub fn cleanup(&self);
}
```

### 10.2 虚拟化架构 (支持Agent任务列表)
```rust
// ui/virtualization.rs 虚拟化架构
pub struct VirtualList<T> {
    items: Vec<T>,
    viewport_height: usize,
    item_height: usize,
    scroll_offset: usize,
    visible_range: Range<usize>,
}

impl<T> VirtualList<T> {
    pub fn render_visible_items(&self) -> &[T];
    pub fn scroll_to(&mut self, index: usize);
    pub fn update_viewport(&mut self, height: usize);
}
```

## 11. 测试架构 (包含Agent和MCP测试)

### 11.1 测试模块结构 (扩展Agent和MCP测试)
```rust
// tests/ 目录架构
tests/
├── unit/                  (单元测试)
│   ├── git_tests.rs
│   ├── ai_tests.rs
│   ├── search_tests.rs
│   └── ui_tests.rs
├── integration/           (集成测试)
│   ├── end_to_end.rs
│   ├── git_integration.rs
│   └── ai_integration.rs
├── fixtures/              (测试数据)
│   ├── git_repos/
│   └── mock_data/
└── helpers/               (测试辅助)
    ├── mock_git.rs
    ├── mock_ai.rs
    └── test_utils.rs
```

这个架构设计确保了代码的模块化、可测试性、可扩展性和高性能，同时提供了清晰的职责分离和良好的错误处理机制。