# Makefile for micropub CLI

.PHONY: help build release test clean install dev check fmt lint run doc all

# Default target
.DEFAULT_GOAL := help

# Binary name
BINARY_NAME := micropub
INSTALL_PATH := ~/.local/bin

help: ## Show this help message
	@echo "micropub CLI - Makefile targets:"
	@echo ""
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-15s\033[0m %s\n", $$1, $$2}'

all: fmt lint test build ## Run fmt, lint, test, and build

build: ## Build debug binary
	cargo build

release: ## Build optimized release binary
	cargo build --release

test: ## Run all tests
	cargo test

check: ## Run cargo check
	cargo check

fmt: ## Format code with rustfmt
	cargo fmt

lint: ## Run clippy linter
	cargo clippy -- -D warnings

doc: ## Generate and open documentation
	cargo doc --open --no-deps

clean: ## Clean build artifacts
	cargo clean
	rm -rf target/

install: release ## Install release binary to ~/.local/bin
	@mkdir -p $(INSTALL_PATH)
	cp target/release/$(BINARY_NAME) $(INSTALL_PATH)/
	@echo "Installed $(BINARY_NAME) to $(INSTALL_PATH)"

uninstall: ## Remove installed binary
	rm -f $(INSTALL_PATH)/$(BINARY_NAME)
	@echo "Uninstalled $(BINARY_NAME) from $(INSTALL_PATH)"

dev: ## Build and run in debug mode
	cargo run --

run: release ## Build and run release binary
	./target/release/$(BINARY_NAME)

watch: ## Watch for changes and rebuild
	cargo watch -x build

watch-test: ## Watch for changes and run tests
	cargo watch -x test

bloat: ## Analyze binary size
	cargo bloat --release

audit: ## Security audit dependencies
	cargo audit

update: ## Update dependencies
	cargo update

bench: ## Run benchmarks
	cargo bench

coverage: ## Generate test coverage report (requires cargo-tarpaulin)
	cargo tarpaulin --out Html --output-dir coverage

deps: ## Show dependency tree
	cargo tree

outdated: ## Check for outdated dependencies (requires cargo-outdated)
	cargo outdated
