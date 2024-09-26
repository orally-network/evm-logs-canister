#!/usr/bin/make
POCKET_IC_BIN := ./pocket-ic
EVM_LOGS_CANISTER_WASM := ./target/wasm32-unknown-unknown/release/evm_logs_canister.wasm.gz


.DEFAULT_GOAL: help

.PHONY: help
help: ## Show this help
	@printf "\033[33m%s:\033[0m\n" 'Available commands'
	@awk 'BEGIN {FS = ":.*?## "} /^[a-zA-Z_-]+:.*?## / {printf "  \033[32m%-18s\033[0m %s\n", $$1, $$2}' $(MAKEFILE_LIST)




.PHONY: build
build: ## Build all canisters
	@./scripts/build

