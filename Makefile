#!/usr/bin/make
POCKET_IC_BIN := ./pocket-ic
EVM_LOGS_CANISTER_WASM := ./target/wasm32-unknown-unknown/release/evm_logs_canister.wasm
TEST_CANISTER_WASM := ./target/wasm32-unknown-unknown/release/test_canister1.wasm
PROXY_CANISTER_WASM := ./target/wasm32-unknown-unknown/release/proxy_canister.wasm
.DEFAULT_GOAL: help

local_deploy_evm_rpc:
	dfx deploy evm_rpc --argument '(record { nodesInSubnet = 28 })'

local_deploy_proxy:
	dfx deploy proxy_canister

local_deploy_test_canister:
	dfx deploy test_canister1

local_deploy: local_deploy_evm_rpc local_deploy_proxy local_deploy_test_canister
	$(eval MAINNET_RPC_URL?=https://eth.llamarpc.com)
	$(eval EVM_RPC_CANISTER := $(shell dfx canister id evm_rpc))
	$(eval PROXY_CANISTER := $(shell dfx canister id proxy_canister))
	$(eval TEST_CANISTER := $(shell dfx canister id test_canister1))

	dfx canister create evm_logs_canister && dfx build evm_logs_canister 
	gzip -f -1 ./.dfx/local/canisters/evm_logs_canister/evm_logs_canister.wasm
	dfx canister install --wasm ./.dfx/local/canisters/evm_logs_canister/evm_logs_canister.wasm.gz --argument \
		"(record { \
			evm_rpc_canister=principal\"${EVM_RPC_CANISTER}\"; \
			proxy_canister=principal\"${PROXY_CANISTER}\"; \
			rpc_wrapper=\"https://rpc.orally.network/?rpc=\";  \
		})" evm_logs_canister \

local_upgrade:
	dfx build evm_logs_canister 
	gzip -f -1 ./.dfx/local/canisters/evm_logs_canister/evm_logs_canister.wasm
	dfx canister install --mode upgrade --wasm ./.dfx/local/canisters/evm_logs_canister/evm_logs_canister.wasm.gz evm_logs_canister

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
	   PROXY_CANISTER_WASM_PATH=$(PROXY_CANISTER_WASM) \
	   POCKET_IC_BIN=$(POCKET_IC_BIN) \
	   RUST_BACKTRACE=1 cargo test $(TEST) --no-fail-fast -- $(if $(TEST_NAME),$(TEST_NAME),) --nocapture


.PHONY: fetch-pocket-ic
fetch-pocket-ic: ## Fetch the pocket-ic binary for tests if not already present
	./scripts/fetch-pocket-ic
