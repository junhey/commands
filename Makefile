.DEFAULT_GOAL := help
.PHONY: help setup dev run test test-cli test-web lint fmt build build-cli build-web site check clean

help: ## 显示可用目标
	@grep -hE '^[a-z-]+:.*?## ' $(MAKEFILE_LIST) | awk -F':.*?## ' '{printf "  \033[36m%-12s\033[0m %s\n", $$1, $$2}'

setup: ## 安装站点依赖
	cd web && npm install

dev: ## 启动站点开发服务（同时提供 /install.sh）
	cd web && npm run dev

run: ## 进入交互式 cmds（debug 构建）
	cargo run

test: test-cli test-web ## 跑全部测试

test-cli: ## CLI 单元测试（含单线程复跑，防并行污染）
	cargo test --locked
	cargo test --locked -- --test-threads=1

test-web: ## Playground 内核测试
	cd web && npm test

lint: ## 格式检查 + clippy + 安装脚本语法
	cargo fmt --all --check
	cargo clippy --all-targets --locked -- -D warnings
	sh -n install/install.sh

fmt: ## 格式化 Rust 代码
	cargo fmt --all

build: build-cli build-web ## 构建 CLI 与站点

build-cli: ## 构建 release 二进制
	cargo build --release --locked
	@ls -lh target/release/cmds

build-web: ## 构建站点到 web/dist
	cd web && npm run build

site: build-web ## 构建并本地预览站点产物
	cd web && npm run preview

check: lint test build ## 提交前完整自查（等价于 CI）
	@echo "全部通过"

clean: ## 清理构建产物
	cargo clean
	rm -rf web/dist
