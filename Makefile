#!/usr/bin/make
POCKET_IC_BIN := ./pocket-ic
EVM_LOGS_CANISTER_WASM := ./target/wasm32-unknown-unknown/release/evm_logs_canister.wasm
TEST_CANISTER_WASM := ./target/wasm32-unknown-unknown/release/test_canister.wasm
PROXY_CANISTER_WASM := ./target/wasm32-unknown-unknown/release/proxy_canister.wasm
CYCLES_WALLET_WASM := ./target/wasm32-unknown-unknown/release/wallet.wasm
EVM_RPC_MOCKED_WASM := ./target/wasm32-unknown-unknown/release/evm_rpc_mocked.wasm

.DEFAULT_GOAL: help

local_deploy: local_deploy_evm_rpc local_deploy_proxy local_deploy_test_canister
	$(eval EVM_RPC_CANISTER := $(shell dfx canister id evm_rpc))
	$(eval PROXY_CANISTER := $(shell dfx canister id proxy_canister))

	dfx canister create evm_logs_canister && dfx build evm_logs_canister

	gzip -f -1 ./.dfx/local/canisters/evm_logs_canister/evm_logs_canister.wasm

	dfx canister install --wasm ./.dfx/local/canisters/evm_logs_canister/evm_logs_canister.wasm.gz --argument \
		"(record { \
			evm_rpc_canister=principal\"${EVM_RPC_CANISTER}\"; \
			proxy_canister=principal\"${PROXY_CANISTER}\"; \
			estimate_events_num = 5:nat32; \
			max_response_bytes = 1000000:nat32 \
		})" evm_logs_canister \

ic_deploy:
	# Create Proxy Canister
	dfx canister create proxy_canister --network ic
	dfx build proxy_canister --ic

	gzip -f -1 ./.dfx/ic/canisters/proxy_canister/proxy_canister.wasm

	# Install Proxy Canister
	dfx canister install proxy_canister --network ic --wasm ./.dfx/ic/canisters/proxy_canister/proxy_canister.wasm.gz

	# Fetch Canister IDs
	$(eval EVM_RPC_CANISTER := $(shell dfx canister id evm_rpc --network ic))
	$(eval PROXY_CANISTER := $(shell dfx canister id proxy_canister --network ic))

	# Create EVM Logs Canister
	dfx canister create evm_logs_canister --network ic
	dfx build evm_logs_canister --ic

	gzip -f -1 ./.dfx/ic/canisters/evm_logs_canister/evm_logs_canister.wasm

	# Install EVM Logs Canister with Arguments
	dfx canister install evm_logs_canister --network ic --wasm ./.dfx/ic/canisters/evm_logs_canister/evm_logs_canister.wasm.gz --argument \
		"(record { \
			evm_rpc_canister = principal \"${EVM_RPC_CANISTER}\"; \
			proxy_canister = principal \"${PROXY_CANISTER}\"; \
			estimate_events_num = 5 : nat32; \
			max_response_bytes = 1000000 : nat32 \
		})"

ic_upgrade:
	dfx build evm_logs_canister --network ic
	gzip -f -1 ./.dfx/ic/canisters/evm_logs_canister/evm_logs_canister.wasm
	dfx canister install --mode upgrade --wasm ./.dfx/ic/canisters/evm_logs_canister/evm_logs_canister.wasm.gz --network ic evm_logs_canister

local_deploy_evm_rpc:
	dfx deploy evm_rpc --argument '(record { nodesInSubnet = 28 })'

local_deploy_proxy:
	dfx deploy proxy_canister

local_deploy_test_canister:
	dfx deploy test_canister

local_deploy_cycles_wallet:
	dfx deploy cycles_wallet


local_test_canister_subscribe:
	$(eval EVM_LOGS_CANISTER := $(shell dfx canister id evm_logs_canister))
	dfx canister call test_canister subscribe '(principal "${EVM_LOGS_CANISTER}")'

local_upgrade:
	dfx build evm_logs_canister 
	gzip -f -1 ./.dfx/local/canisters/evm_logs_canister/evm_logs_canister.wasm
	dfx canister install --mode upgrade --wasm ./.dfx/local/canisters/evm_logs_canister/evm_logs_canister.wasm.gz evm_logs_canister

	dfx build test_canister
	gzip -f -1 ./.dfx/local/canisters/test_canister/test_canister.wasm
	dfx canister install --mode upgrade --wasm ./.dfx/local/canisters/test_canister/test_canister.wasm.gz test_canister

.PHONY: help
help: ## Show this help
	@printf "\033[33m%s:\033[0m\n" 'Available commands'
	@awk 'BEGIN {FS = ":.*?## "} /^[a-zA-Z_-]+:.*?## / {printf "  \033[32m%-18s\033[0m %s\n", $$1, $$2}' $(MAKEFILE_LIST)


.PHONY: build
build: ## Build all canisters
	cargo build --release --target wasm32-unknown-unknown --package evm_logs_canister
	cargo build --release --target wasm32-unknown-unknown --package test_canister
	cargo build --release --target wasm32-unknown-unknown --package evm_rpc_mocked
	cargo build --release --target wasm32-unknown-unknown --package proxy_canister

.PHONY: test
test: build ## Run tests
	@echo "Running tests..."
	@if [ ! -f "$(POCKET_IC_BIN)" ]; then \
		echo "Pocket IC binary not found. Fetching..."; \
		$(MAKE) fetch-pocket-ic; \
	fi
	@EVM_LOGS_CANISTER_PATH=$(EVM_LOGS_CANISTER_WASM) \
	   TEST_CANISTER_WASM_PATH=$(TEST_CANISTER_WASM) \
	   CYCLES_WALLET_WASM_PATH=$(CYCLES_WALLET_WASM) \
	   PROXY_CANISTER_WASM_PATH=$(PROXY_CANISTER_WASM) \
	   EVM_RPC_MOCKED_WASM_PATH=$(EVM_RPC_MOCKED_WASM) \
	   POCKET_IC_BIN=$(POCKET_IC_BIN) \
	   cargo test $(TEST) --no-fail-fast -- $(if $(TEST_NAME),$(TEST_NAME),) --nocapture	


.PHONY: fetch-pocket-ic
fetch-pocket-ic: ## Fetch the pocket-ic binary for tests if not already present
	./scripts/fetch-pocket-ic
