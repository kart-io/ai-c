# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

AI-C TUI is an intelligent Git commit tool built in Rust with a Terminal User Interface. The project features a sophisticated Agent architecture for AI services and Model Context Protocol (MCP) integration for external AI service communication.

## Essential Commands

```bash
# Build and run
make build                    # Build debug version
make build-release           # Build optimized release version
make run                     # Run the application
cargo run -- <args>          # Run with command-line arguments

# Testing
make test                    # Run all tests
make test-core              # Run core functionality tests
make test-unit              # Run unit tests only
make test-watch             # Auto-run tests on file changes
cargo test git_tests        # Run specific test module
cargo test --lib            # Run library unit tests

# Code quality
make fmt                     # Format code with rustfmt
make clippy                  # Run clippy linting with strict warnings
make check                   # Quick validation without building
make pre-commit             # Run format, clippy, and tests before committing

# Performance and documentation
make bench                   # Run performance benchmarks
make coverage               # Generate test coverage report
make doc                     # Generate API documentation
make size                    # Analyze binary size
```

## High-Level Architecture

The project implements a layered architecture with three major systems:

### Core Systems

1. **Agent System** (`src/ai/`)
   - `AgentManager`: Centralized lifecycle management and orchestration
   - `MessageBus`: Async inter-agent communication via tokio channels
   - Specialized agents: `CommitAgent`, `AnalysisAgent`, `ReviewAgent`, `SearchAgent`
   - Hot-swappable agent modules with health monitoring

2. **MCP Protocol Integration** (`src/ai/mcp/`)
   - Full MCP 1.0 specification implementation
   - Transport layers: WebSocket, HTTP, stdio
   - Dynamic resource discovery and tool registration
   - Protocol negotiation and error handling

3. **Git Service Layer** (`src/git/`)
   - High-performance Git operations with intelligent caching
   - Supports repositories with >10,000 files and >50,000 commits
   - File status refresh target: <200ms
   - Git Flow workflow management

### Application Structure

- **State Management** (`src/app/state.rs`): Centralized AppState with reactive updates
- **Event System** (`src/app/events.rs`): Event-driven architecture using tokio channels
- **UI Framework** (`src/ui/`): TUI built with ratatui, six main tabs, three-column layout
- **Configuration** (`src/config/`): TOML-based with hot reloading support

## Performance Requirements

- Application startup: < 1 second
- Agent system initialization: < 500ms
- File status refresh: < 200ms
- MCP protocol communication: < 100ms
- Memory usage: < 150MB idle, < 600MB with multiple agents

## Testing Infrastructure

Test files are organized by functionality:
- `tests/git_tests.rs` - Git operations and caching
- `tests/commit_agent_tests.rs` - Agent system tests
- `tests/ai_client_tests.rs` - AI service integration
- `tests/integration_tests.rs` - End-to-end workflows
- `tests/core_test.rs` - Core functionality validation

Coverage target: ≥80% overall, ≥95% for core modules

## Development Workflow

The project follows a Three-Layer Guidance Framework documented in `docs/开发任务列表.md`:

1. **📋 需求依据** - Requirements from design documents
2. **🏗️ 技术指导** - Technical implementation guidance
3. **🔗 参考实现** - Code patterns and examples

Always verify implementations against requirements in `docs/` directory.

## Key Design Patterns

### Agent Development
```rust
// Agents must implement the Agent trait
#[async_trait]
impl Agent for YourAgent {
    async fn process(&mut self, message: Message) -> Result<Response>;
    async fn health_check(&self) -> HealthStatus;
}
```

### MCP Message Format
```rust
// Follow JSON-RPC 2.0 specification
MpcMessage {
    jsonrpc: "2.0",
    method: String,
    params: Option<Value>,
    id: Option<Value>,
}
```

### Error Handling
- Use `thiserror` for custom errors
- Use `anyhow` for error propagation
- All public APIs return `Result<T, Error>`

## Configuration

Configuration files use TOML format:
- `config/default.toml` - Base configuration
- `config/development.toml` - Development overrides
- `config/production.toml` - Production settings

Environment-specific configs are loaded automatically based on `APP_ENV` variable.

## Critical Implementation Notes

### Git Operations
- Cache invalidation on repository changes
- Batch operations for performance
- Handle large repositories (>10K files) efficiently

### Agent System
- Message ordering guarantees via channel sequencing
- Automatic retry with exponential backoff
- Circuit breaker pattern for failing agents

### MCP Protocol
- Support multiple concurrent connections
- Resource access control via capability system
- Tool execution sandboxing

### UI Responsiveness
- Non-blocking Git operations
- Progressive rendering for large datasets
- Debounced input handling