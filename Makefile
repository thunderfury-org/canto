CARGO ?= cargo

.DEFAULT_GOAL := help

.PHONY: help build build-release check test clippy fmt fmt-check lint verify netns-check clean

help: ## 显示可用命令及说明
	@awk 'BEGIN {FS = ":.*?## "}; /^[a-zA-Z_-]+:.*?## / {printf "\033[36m%-16s\033[0m %s\n", $$1, $$2}' $(MAKEFILE_LIST) | sort

build: ## 编译 Debug 调试版本
	$(CARGO) build

build-release: ## 编译 Release 优化版本
	$(CARGO) build --release

check: ## 快速检查语法与类型编译
	$(CARGO) check

test: ## 运行所有单元测试
	$(CARGO) test

clippy: ## 执行 Clippy 静态代码检查 (告警视为错误)
	$(CARGO) clippy --all-targets --all-features -- -D warnings

fmt: ## 自动格式化 Rust 源代码
	$(CARGO) fmt

fmt-check: ## 检查代码格式规范
	$(CARGO) fmt -- --check

lint: fmt-check clippy ## 综合代码风格与静态质量检查

verify: lint test ## 运行提交前综合验证 (lint + test)

netns-check: ## 网关 netns 全量检查（Linux root：tproxy、URL 源、刷新、缓存）
	./scripts/netns-check.sh

clean: ## 清理构建产物与缓存
	$(CARGO) clean
