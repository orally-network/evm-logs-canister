#!/usr/bin/make
POCKET_IC_BIN := ./pocket-ic
EVM_LOGS_CANISTER_WASM := ./target/wasm32-unknown-unknown/release/evm_logs_canister.wasm
TEST_CANISTER_WASM := ./target/wasm32-unknown-unknown/release/test_canister.wasm
.DEFAULT_GOAL: help

.PHONY: help
help: ## Show this help
	@printf "\033[33m%s:\033[0m\n" 'Available commands'
	@awk 'BEGIN {FS = ":.*?## "} /^[a-zA-Z_-]+:.*?## / {printf "  \033[32m%-18s\033[0m %s\n", $$1, $$2}' $(MAKEFILE_LIST)

.PHONY: build
build: ## Build all canisters
	@./scripts/build build

.PHONY: deploy
deploy: ## Deploy all canisters
	@./scripts/build deploy

.PHONY: test
test: build ## Run tests
	@echo "Running tests..."
	@if [ ! -f "$(POCKET_IC_BIN)" ]; then \
		echo "Pocket IC binary not found. Fetching..."; \
		$(MAKE) fetch-pocket-ic; \
	fi
	@EVM_LOGS_CANISTER_PATH=$(EVM_LOGS_CANISTER_WASM) \
	   TEST_CANISTER_WASM_PATH=$(TEST_CANISTER_WASM) \
	   POCKET_IC_BIN=$(POCKET_IC_BIN) \
	   cargo test $(TEST) --no-fail-fast -- $(if $(TEST_NAME),$(TEST_NAME),) --nocapture

.PHONY: fetch-pocket-ic
fetch-pocket-ic: ## Fetch the pocket-ic binary for tests if not already present
	./scripts/fetch-pocket-ic
