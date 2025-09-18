# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

AI-C TUI is an intelligent Git commit tool built in Rust with a Terminal User Interface. The project features a sophisticated Agent architecture for AI services and Model Context Protocol (MCP) integration for external AI service communication.

## Documentation Structure

The project documentation is organized as follows:

- `docs/开发任务列表.md` - Development task list with 16-week timeline
- `docs/Agent架构技术设计文档.md` - Agent system architecture and implementation specs
- `docs/MCP协议集成技术设计文档.md` - Model Context Protocol integration design
- `docs/核心接口和数据结构定义文档.md` - Core interfaces and data structure definitions
- `docs/Rust依赖包列表文档.md` - Complete dependency list
- `docs/Rust项目文件布局文档.md` - Project file structure
- `docs/单元测试文档.md` - Comprehensive testing strategy

## Development Commands

The project includes a comprehensive Makefile with development commands. Use `make help` to see all available commands.

### Essential Commands

```bash
# Quick development cycle
make help           # Show all available commands
make build          # Build project (debug)
make run            # Run the application
make test           # Run all tests
make clean          # Clean build artifacts

# Code quality
make fmt            # Format code with rustfmt
make clippy         # Run clippy linting
make check          # Quick code check without building
make pre-commit     # Run format, clippy, and tests

# Testing options
make test-unit      # Run unit tests only
make test-core      # Run core functionality tests
make test-minimal   # Run minimal test suite
make test-watch     # Watch files and auto-run tests

# Documentation and analysis
make doc            # Generate documentation
make coverage       # Generate test coverage report
make size           # Show binary size analysis
make lines          # Count lines of code
```

### Direct Cargo Commands

```bash
# Standard Rust commands also work
cargo build --release
cargo test git_tests
cargo test commit_agent_tests
cargo clippy -- -D warnings
cargo fmt
cargo check
```

## High-Level Architecture

### Core Architecture Layers

1. **Presentation Layer**: TUI interface built with `ratatui` and `crossterm`
   - Six main tabs: Branches, Tags, Stash, Status, Remotes, Git工作流
   - Three-column layout: Sidebar, Main Content, Detail Panel
   - Responsive design adapting to terminal size

2. **Application Layer**: Event handling and state management
   - Centralized `AppState` with reactive updates
   - Event-driven architecture using `tokio` channels
   - Command routing and action dispatching

3. **Domain Layer**: Core business logic with three major systems:
   - **Git Workflow Management**: Branch, tag, remote, and Git Flow operations
   - **Agent System**: AI service management with message bus architecture
   - **MCP Protocol**: Model Context Protocol for AI service communication

4. **Infrastructure Layer**: External integrations and system services

### Agent Architecture System

The project implements a sophisticated Agent-based AI system:

- **Agent Manager**: Lifecycle management, task distribution, health monitoring
- **Message Bus**: Asynchronous inter-agent communication using `tokio` channels
- **Specialized Agents**:
  - `CommitAgent`: Generates commit messages from staged changes
  - `AnalysisAgent`: Performs code analysis and impact assessment
  - `ReviewAgent`: Automated code review and quality checks
  - `SearchAgent`: Semantic search and query optimization

### MCP Protocol Integration

Model Context Protocol support enables external AI service integration:

- **Protocol Implementation**: Full MCP 1.0 specification support
- **Transport Layer**: WebSocket, HTTP, and stdio transport options
- **Resource Management**: Dynamic resource discovery and access control
- **Tool System**: Extensible tool registration and execution framework

### Key Design Patterns

1. **Three-Layer Guidance Framework**: Each development task follows:
   - 📋 需求依据 (Requirements Basis)
   - 🏗️ 技术指导 (Technical Guidance)
   - 🔗 参考实现 (Reference Implementation)

2. **Document Verification**: Every major task includes verification steps against requirements documentation

3. **Modular Agent Design**: Each agent is self-contained with clear capabilities and communication interfaces

## Critical Development Notes

### Actual File Structure

Current implementation structure:

- `src/app/`: Application core with state management and event handling
- `src/ui/`: Terminal user interface components built with ratatui
- `src/git/`: Git operations service with caching and performance optimization
- `src/ai/`: Agent system with message bus architecture
- `src/config/`: Configuration management with TOML support
- `tests/`: Comprehensive test suite including unit, integration, and agent tests

### Performance Requirements

- Application startup: < 1 second
- Agent system initialization: < 500ms
- File status refresh: < 200ms
- MCP protocol communication: < 100ms
- Memory usage: < 150MB idle, < 600MB with multiple agents
- Support for repositories with >10,000 files and >50,000 commits

### Testing Strategy

The project includes comprehensive testing infrastructure:

- **Test Files Available**: `git_tests.rs`, `commit_agent_tests.rs`, `ai_client_tests.rs`, `integration_tests.rs`, `core_test.rs`
- **Coverage Target**: ≥80% overall, ≥95% for core modules
- **Test Commands**: Use `make test` for all tests, `make test-core` for core functionality
- **Performance Testing**: `make bench` for benchmarks, `make coverage` for coverage reports
- **Continuous Testing**: `make test-watch` for development with auto-reload

### Configuration Management

- TOML-based configuration system
- Hot reloading for all configuration changes
- Environment-specific configs (dev/test/prod)
- Separate configuration for AI providers, agents, and MCP services

## Development Workflow

### Task Execution Pattern

When implementing features, always follow this pattern from the development task list:

1. **Requirements Review**: Check the 📋 需求依据 section for specific requirements
2. **Technical Implementation**: Follow the 🏗️ 技术指导 for architectural guidance
3. **Reference Implementation**: Use 🔗 参考实现 for implementation patterns
4. **Documentation Verification**: Complete 📚 文档验证 step to ensure requirements compliance

### Documentation Organization Requirements

When generating code or documentation, structure content according to these guidelines:

#### 1. **文档组织优化**
- 📋 **快速开始指南** (30分钟上手) - 提供简洁的入门路径和核心概念
- 📋 **核心架构概览** (Agent + MCP系统) - 高层架构图和关键组件说明
- 📋 **详细实现指南** (当前内容) - 完整的开发任务和技术细节
- 📋 **故障排查和优化指南** - 常见问题解决和性能调优

#### 2. **技术决策细化**
- **关键技术选型标准**: 提供具体的判断依据（性能、维护性、生态系统）
- **边界情况处理**: 详细说明错误处理、资源限制、并发冲突等情况
- **性能调优策略**: 具体的优化方法、监控指标、调试技巧

#### 3. **风险监控强化**
- **实时监控机制**: 建立关键指标的自动监控和告警系统
- **应急方案实施**: 详细的故障响应步骤和恢复流程
- **团队技能评估**: 明确各阶段所需的技术能力和学习要求

#### 4. **学习路径优化**
- **分层指导**: 为初级/中级/高级开发者提供不同深度的指导
- **概念解释**: 对复杂技术概念提供清晰的解释和实例
- **渐进式学习**: 按照难度递增安排学习内容和实践项目

### Code Generation Guidelines

When generating code, ensure all outputs follow these principles:
- Include comprehensive error handling for edge cases
- Add performance monitoring and logging where appropriate
- Provide clear documentation and inline comments
- Structure code to support future optimization and scaling
- Include relevant test cases and benchmarks

### Agent Development

When creating new agents:
- Implement the `Agent` trait with required methods
- Register with `AgentManager` during application startup
- Use the message bus for inter-agent communication
- Include comprehensive error handling and logging

### MCP Integration

When adding MCP functionality:
- Follow JSON-RPC 2.0 specification for message format
- Implement proper resource and tool registration
- Support multiple transport layers (WebSocket preferred)
- Include protocol version negotiation and error handling

## Quick Start for New Contributors

```bash
# Setup and first run
make help           # See all available commands
make dev-setup      # Install development tools (first time only)
make build          # Build the project
make test           # Verify everything works
make run            # Start the application

# Development workflow
make check          # Quick code validation
make test-watch     # Start continuous testing
# ... make changes ...
make pre-commit     # Before committing changes
```

This project emphasizes documentation-driven development with comprehensive verification steps to ensure all features meet the detailed requirements specifications.