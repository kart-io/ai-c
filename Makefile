# AI-Commit TUI Project Makefile
#
# This Makefile provides convenient commands for development, testing, and deployment
# of the AI-Commit TUI application.

# =============================================================================
# Project Configuration
# =============================================================================

# Project metadata
PROJECT_NAME := ai-c
VERSION := $(shell grep '^version' Cargo.toml | head -n1 | cut -d'"' -f2)
RUST_VERSION := $(shell rustc --version)

# Build configurations
BUILD_DIR := target
RELEASE_DIR := $(BUILD_DIR)/release
DEBUG_DIR := $(BUILD_DIR)/debug

# Test configurations
COVERAGE_DIR := $(BUILD_DIR)/coverage
BENCHMARK_DIR := $(BUILD_DIR)/criterion

# Colors for output
GREEN := \033[32m
YELLOW := \033[33m
RED := \033[31m
BLUE := \033[34m
RESET := \033[0m

# =============================================================================
# Help & Information
# =============================================================================

.PHONY: help
help: ## 显示所有可用命令
	@echo "$(BLUE)AI-C TUI Development Commands$(RESET)"
	@echo "Version: $(VERSION)"
	@echo "Rust: $(RUST_VERSION)"
	@echo ""
	@echo "$(GREEN)Basic Commands:$(RESET)"
	@awk 'BEGIN {FS = ":.*?## "} /^[a-zA-Z_-]+:.*?## / {printf "  $(YELLOW)%-15s$(RESET) %s\n", $$1, $$2}' $(MAKEFILE_LIST) | grep -E "(run|build|test|clean)"
	@echo ""
	@echo "$(GREEN)Development Commands:$(RESET)"
	@awk 'BEGIN {FS = ":.*?## "} /^[a-zA-Z_-]+:.*?## / {printf "  $(YELLOW)%-15s$(RESET) %s\n", $$1, $$2}' $(MAKEFILE_LIST) | grep -E "(check|fmt|clippy|fix)"
	@echo ""
	@echo "$(GREEN)Advanced Commands:$(RESET)"
	@awk 'BEGIN {FS = ":.*?## "} /^[a-zA-Z_-]+:.*?## / {printf "  $(YELLOW)%-15s$(RESET) %s\n", $$1, $$2}' $(MAKEFILE_LIST) | grep -vE "(run|build|test|clean|check|fmt|clippy|fix)"

.PHONY: info
info: ## 显示项目信息
	@echo "$(BLUE)Project Information$(RESET)"
	@echo "Name: $(PROJECT_NAME)"
	@echo "Version: $(VERSION)"
	@echo "Rust Version: $(RUST_VERSION)"
	@echo "Build Directory: $(BUILD_DIR)"
	@echo "Debug Target: $(DEBUG_DIR)/$(PROJECT_NAME)"
	@echo "Release Target: $(RELEASE_DIR)/$(PROJECT_NAME)"

# =============================================================================
# Build Commands
# =============================================================================

.PHONY: build
build: ## 构建项目 (debug模式)
	@echo "$(GREEN)Building project in debug mode...$(RESET)"
	@cargo build

.PHONY: build-release
build-release: ## 构建项目 (release模式)
	@echo "$(GREEN)Building project in release mode...$(RESET)"
	@cargo build --release

.PHONY: rebuild
rebuild: clean build ## 清理后重新构建

.PHONY: rebuild-release
rebuild-release: clean build-release ## 清理后重新构建 (release模式)

# =============================================================================
# Run Commands
# =============================================================================

.PHONY: run
run: ## 运行项目 (debug模式)
	@echo "$(GREEN)Running project in debug mode...$(RESET)"
	@cargo run

.PHONY: run-release
run-release: ## 运行项目 (release模式)
	@echo "$(GREEN)Running project in release mode...$(RESET)"
	@cargo run --release

.PHONY: run-verbose
run-verbose: ## 运行项目 (详细输出)
	@echo "$(GREEN)Running project with verbose output...$(RESET)"
	@RUST_LOG=debug cargo run

# =============================================================================
# Test Commands
# =============================================================================

.PHONY: test
test: ## 运行所有测试
	@echo "$(GREEN)Running all tests...$(RESET)"
	@cargo test

.PHONY: test-verbose
test-verbose: ## 运行测试 (详细输出)
	@echo "$(GREEN)Running tests with verbose output...$(RESET)"
	@cargo test -- --nocapture

.PHONY: test-unit
test-unit: ## 运行单元测试
	@echo "$(GREEN)Running unit tests...$(RESET)"
	@cargo test --lib

.PHONY: test-integration
test-integration: ## 运行集成测试
	@echo "$(GREEN)Running integration tests...$(RESET)"
	@cargo test --test integration_tests

.PHONY: test-basic
test-basic: ## 运行基础测试
	@echo "$(GREEN)Running basic tests...$(RESET)"
	@cargo test --test basic_tests

.PHONY: test-core
test-core: ## 运行核心测试
	@echo "$(GREEN)Running core tests...$(RESET)"
	@cargo test --test core_test

.PHONY: test-minimal
test-minimal: ## 运行最小测试集
	@echo "$(GREEN)Running minimal tests...$(RESET)"
	@cargo test --test minimal_test

.PHONY: test-watch
test-watch: ## 监视文件变化并自动运行测试
	@echo "$(GREEN)Watching for changes and running tests...$(RESET)"
	@cargo watch -x test

# =============================================================================
# Code Quality Commands
# =============================================================================

.PHONY: check
check: ## 检查代码 (不编译)
	@echo "$(GREEN)Checking code...$(RESET)"
	@cargo check

.PHONY: check-all
check-all: ## 检查所有目标代码
	@echo "$(GREEN)Checking all targets...$(RESET)"
	@cargo check --all-targets

.PHONY: clippy
clippy: ## 运行Clippy代码检查
	@echo "$(GREEN)Running Clippy lints...$(RESET)"
	@cargo clippy -- -D warnings

.PHONY: clippy-fix
clippy-fix: ## 自动修复Clippy问题
	@echo "$(GREEN)Auto-fixing Clippy issues...$(RESET)"
	@cargo clippy --fix --allow-dirty --allow-staged

.PHONY: fmt
fmt: ## 格式化代码
	@echo "$(GREEN)Formatting code...$(RESET)"
	@cargo fmt

.PHONY: fmt-check
fmt-check: ## 检查代码格式
	@echo "$(GREEN)Checking code formatting...$(RESET)"
	@cargo fmt -- --check

.PHONY: fix
fix: ## 自动修复代码问题
	@echo "$(GREEN)Auto-fixing code issues...$(RESET)"
	@cargo fix --allow-dirty --allow-staged

# =============================================================================
# Documentation Commands
# =============================================================================

.PHONY: doc
doc: ## 生成文档
	@echo "$(GREEN)Generating documentation...$(RESET)"
	@cargo doc --no-deps

.PHONY: doc-open
doc-open: ## 生成并打开文档
	@echo "$(GREEN)Generating and opening documentation...$(RESET)"
	@cargo doc --no-deps --open

.PHONY: doc-private
doc-private: ## 生成包含私有项的文档
	@echo "$(GREEN)Generating documentation (including private items)...$(RESET)"
	@cargo doc --no-deps --document-private-items

# =============================================================================
# Benchmark Commands
# =============================================================================

.PHONY: bench
bench: ## 运行性能基准测试
	@echo "$(GREEN)Running benchmarks...$(RESET)"
	@cargo bench

.PHONY: bench-git
bench-git: ## 运行Git操作基准测试
	@echo "$(GREEN)Running Git operation benchmarks...$(RESET)"
	@cargo bench --bench git_benchmarks

.PHONY: bench-agent
bench-agent: ## 运行Agent系统基准测试
	@echo "$(GREEN)Running Agent system benchmarks...$(RESET)"
	@cargo bench --bench agent_benchmarks

# =============================================================================
# Coverage Commands
# =============================================================================

.PHONY: coverage
coverage: ## 生成测试覆盖率报告
	@echo "$(GREEN)Generating coverage report...$(RESET)"
	@mkdir -p $(COVERAGE_DIR)
	@cargo tarpaulin --out Html --output-dir $(COVERAGE_DIR)
	@echo "Coverage report generated at: $(COVERAGE_DIR)/tarpaulin-report.html"

.PHONY: coverage-xml
coverage-xml: ## 生成XML格式覆盖率报告
	@echo "$(GREEN)Generating XML coverage report...$(RESET)"
	@mkdir -p $(COVERAGE_DIR)
	@cargo tarpaulin --out Xml --output-dir $(COVERAGE_DIR)

# =============================================================================
# Dependency Commands
# =============================================================================

.PHONY: deps
deps: ## 显示依赖树
	@echo "$(GREEN)Displaying dependency tree...$(RESET)"
	@cargo tree

.PHONY: deps-update
deps-update: ## 更新依赖
	@echo "$(GREEN)Updating dependencies...$(RESET)"
	@cargo update

.PHONY: deps-outdated
deps-outdated: ## 检查过期依赖
	@echo "$(GREEN)Checking for outdated dependencies...$(RESET)"
	@cargo outdated

.PHONY: deps-audit
deps-audit: ## 安全审计依赖
	@echo "$(GREEN)Auditing dependencies for security issues...$(RESET)"
	@cargo audit

# =============================================================================
# Install Commands
# =============================================================================

.PHONY: install
install: build-release ## 安装到系统
	@echo "$(GREEN)Installing $(PROJECT_NAME)...$(RESET)"
	@cargo install --path . --force

.PHONY: uninstall
uninstall: ## 从系统卸载
	@echo "$(GREEN)Uninstalling $(PROJECT_NAME)...$(RESET)"
	@cargo uninstall $(PROJECT_NAME)

# =============================================================================
# Development Setup Commands
# =============================================================================

.PHONY: dev-setup
dev-setup: ## 安装开发工具
	@echo "$(GREEN)Setting up development environment...$(RESET)"
	@rustup component add clippy rustfmt
	@cargo install cargo-watch cargo-tarpaulin cargo-outdated cargo-audit
	@echo "$(GREEN)Development environment setup complete!$(RESET)"

.PHONY: pre-commit
pre-commit: fmt clippy test ## 预提交检查 (格式化、Clippy、测试)
	@echo "$(GREEN)Pre-commit checks completed successfully!$(RESET)"

# =============================================================================
# Cleaning Commands
# =============================================================================

.PHONY: clean
clean: ## 清理构建文件
	@echo "$(GREEN)Cleaning build files...$(RESET)"
	@cargo clean

.PHONY: clean-all
clean-all: clean ## 清理所有生成的文件
	@echo "$(GREEN)Cleaning all generated files...$(RESET)"
	@rm -rf $(COVERAGE_DIR) $(BENCHMARK_DIR)

# =============================================================================
# Release Commands
# =============================================================================

.PHONY: release-check
release-check: ## 发布前检查
	@echo "$(GREEN)Performing release checks...$(RESET)"
	@cargo check --release
	@cargo clippy --release -- -D warnings
	@cargo test --release
	@echo "$(GREEN)Release checks completed successfully!$(RESET)"

.PHONY: release-build
release-build: ## 构建发布版本
	@echo "$(GREEN)Building release version...$(RESET)"
	@cargo build --release
	@echo "Release binary: $(RELEASE_DIR)/$(PROJECT_NAME)"

# =============================================================================
# Performance Commands
# =============================================================================

.PHONY: perf-test
perf-test: ## 运行性能测试
	@echo "$(GREEN)Running performance tests...$(RESET)"
	@cargo test --release -- --ignored perf_

.PHONY: memory-check
memory-check: build ## 检查内存使用
	@echo "$(GREEN)Checking memory usage with Valgrind...$(RESET)"
	@valgrind --tool=memcheck --leak-check=full ./$(DEBUG_DIR)/$(PROJECT_NAME) || echo "Valgrind not available"

# =============================================================================
# Utility Commands
# =============================================================================

.PHONY: size
size: build-release ## 显示二进制文件大小
	@echo "$(GREEN)Binary size analysis:$(RESET)"
	@ls -lh $(RELEASE_DIR)/$(PROJECT_NAME) 2>/dev/null || echo "Release binary not found. Run 'make build-release' first."
	@echo "$(YELLOW)Debug binary:$(RESET)"
	@ls -lh $(DEBUG_DIR)/$(PROJECT_NAME) 2>/dev/null || echo "Debug binary not found. Run 'make build' first."

.PHONY: lines
lines: ## 统计代码行数
	@echo "$(GREEN)Code line statistics:$(RESET)"
	@find src -name "*.rs" -exec wc -l {} + | sort -n
	@echo "$(YELLOW)Total Rust lines:$(RESET)"
	@find src -name "*.rs" -exec cat {} \; | wc -l

.PHONY: todos
todos: ## 查找代码中的TODO和FIXME
	@echo "$(GREEN)Finding TODOs and FIXMEs:$(RESET)"
	@grep -rn "TODO\|FIXME\|XXX" src/ || echo "No TODOs or FIXMEs found!"

# =============================================================================
# Docker Commands (for future use)
# =============================================================================

.PHONY: docker-build
docker-build: ## 构建Docker镜像 (预留)
	@echo "$(YELLOW)Docker support not yet implemented$(RESET)"

.PHONY: docker-run
docker-run: ## 运行Docker容器 (预留)
	@echo "$(YELLOW)Docker support not yet implemented$(RESET)"

# =============================================================================
# Default target
# =============================================================================

.DEFAULT_GOAL := help

# Make sure we don't have any conflicts with file names
.PHONY: all
all: clean fmt clippy test build ## 执行完整的构建流程

# =============================================================================
# Performance Monitoring
# =============================================================================

.PHONY: monitor
monitor: ## 监控项目构建和测试
	@echo "$(GREEN)Starting project monitoring...$(RESET)"
	@cargo watch -x "check --message-format short" -x "test --quiet"