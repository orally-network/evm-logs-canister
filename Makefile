#!/usr/bin/make
POCKET_IC_BIN := ./pocket-ic
POCKET_IC_BIN_TESTING_PATH := ./../pocket-ic
POCKET_IC_NAME := pocket-ic
EVM_LOGS_CANISTER_WASM := ./../target/wasm32-unknown-unknown/release/evm_logs_canister.wasm
TEST_CANISTER_WASM := ./../target/wasm32-unknown-unknown/release/test_canister.wasm
PROXY_CANISTER_WASM := ./../target/wasm32-unknown-unknown/release/proxy_canister.wasm
CYCLES_WALLET_WASM := ./../target/wasm32-unknown-unknown/release/wallet.wasm
EVM_RPC_MOCKED_WASM := ./../target/wasm32-unknown-unknown/release/evm_rpc_mocked.wasm
FETCH_POCKET_IC_BIN_PATH := ./scripts/fetch-pocket-ic
WALLET_WASM_URL := https://github.com/dfinity/cycles-wallet/releases/download/20240410/wallet.wasm
WALLET_WASM_PATH := ./target/wasm32-unknown-unknown/release/wallet.wasm
DFX_PATH := .dfx

.DEFAULT_GOAL: help

.PHONY: test
.PHONY: help
.PHONY: build
.PHONY: fetch-pocket-ic
.PHONY: fetch-wallet-wasm



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

## Builds 'evm_logs_canister' canister
evm_logs_canister/build:
	dfx build evm_logs_canister

## Builds 'test_canister' canister
test_canister/build:
	dfx build test_canister

## Builds 'evm_rpc_mocked' canister
evm_rpc_mocked/build:
	dfx build evm_rpc_mocked

## Builds 'proxy_canister' canister
proxy_canister/build:
	dfx build proxy_canister

## Updates candid interface file for 'staking_protocol' (has to be installed before https://internetcomputer.org/docs/current/developer-docs/backend/rust/generating-candid)
evm_logs_canister/update_candid: evm_logs_canister/build
	$(eval EVM_LOGS_CAN_NAME := "evm_logs_canister")
	candid-extractor ./target/wasm32-unknown-unknown/release/$(EVM_LOGS_CAN_NAME).wasm > ./evm_logs_canister/$(EVM_LOGS_CAN_NAME).did

## Updates candid interface file for 'staking_protocol' (has to be installed before https://internetcomputer.org/docs/current/developer-docs/backend/rust/generating-candid)
test_canister/update_candid: test_canister/build
	$(eval TEST_CAN_NAME := "test_canister")
	candid-extractor ./target/wasm32-unknown-unknown/release/$(TEST_CAN_NAME).wasm > ./test_canister/$(TEST_CAN_NAME).did

## Updates candid interface file for 'staking_protocol' (has to be installed before https://internetcomputer.org/docs/current/developer-docs/backend/rust/generating-candid)
evm_rpc_mocked/update_candid:  evm_rpc_mocked/build
	$(eval EVM_RPC_CAN_NAME := "evm_rpc_mocked")
	candid-extractor ./target/wasm32-unknown-unknown/release/$(EVM_RPC_CAN_NAME).wasm > ./evm_rpc_mocked/$(EVM_RPC_CAN_NAME).did

## Updates candid interface file for 'staking_protocol' (has to be installed before https://internetcomputer.org/docs/current/developer-docs/backend/rust/generating-candid)
proxy_canister/update_candid: proxy_canister/build
	$(eval PROXY_CAN_NAME := "proxy_canister")
	candid-extractor ./target/wasm32-unknown-unknown/release/$(PROXY_CAN_NAME).wasm > ./proxy_canister/$(PROXY_CAN_NAME).did

## Updates did files for all canisters
local_update_candid: evm_logs_canister/update_candid test_canister/update_candid proxy_canister/update_candid

## Build all canisters
build:
	cargo build --release --target wasm32-unknown-unknown --package evm_logs_canister
	cargo build --release --target wasm32-unknown-unknown --package test_canister
	cargo build --release --target wasm32-unknown-unknown --package evm_rpc_mocked
	cargo build --release --target wasm32-unknown-unknown --package proxy_canister

## Run tests
test: build fetch-wallet-wasm
	@echo "Running tests..."
	@if [ ! -f "$(POCKET_IC_BIN)" ]; then \
		echo "Pocket IC binary not found. Fetching..."; \
		$(MAKE) fetch-pocket-ic; \
	fi
	@POCKET_IC_BIN=$(POCKET_IC_BIN_TESTING_PATH) \
	   cargo test $(TEST) --no-fail-fast -- $(if $(TEST_NAME),$(TEST_NAME),) --nocapture	

help:
	@printf "Available targets:\n\n"
	@awk '/^[a-zA-Z\-\_0-9%:\\]+/ { \
          helpMessage = match(lastLine, /^## (.*)/); \
          if (helpMessage) { \
            helpCommand = $$1; \
            helpMessage = substr(lastLine, RSTART + 3, RLENGTH); \
      gsub("\\\\", "", helpCommand); \
      gsub(":+$$", "", helpCommand); \
            printf "  \x1b[32;01m%-35s\x1b[0m %s\n", helpCommand, helpMessage; \
          } \
        } \
        { lastLine = $$0 }' $(MAKEFILE_LIST) | sort -u
	@printf "\n"

## Fetch the pocket-ic binary for tests if not already present
fetch-pocket-ic:
	chmod +x $(FETCH_POCKET_IC_BIN_PATH)
	$(FETCH_POCKET_IC_BIN_PATH)

## Fetch the wallet.wasm file
fetch-wallet-wasm:
	@mkdir -p $(dir $(WALLET_WASM_PATH))
	curl -sL -o $(WALLET_WASM_PATH) $(WALLET_WASM_URL)
	@echo "wallet.wasm downloaded to $(WALLET_WASM_PATH)"

## Cleans whole directory from temporary files
clean:
	cargo clean
	rm -rf $(DFX_PATH)
	rm -f $(POCKET_IC_BIN_NAME)
