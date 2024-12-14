# This imports the environment variables from the .env file
include .env
export $(shell sed 's/=.*//' .env)
############################# HELP MESSAGE #############################
# Make sure the help command stays first, so that it's printed by default when `make` is called without arguments
.PHONY: help tests build-and-test generate-types deploy add-asset pause unpause sanity-check transfer-owner migrate-v2
help:
	@grep -E '^[a-zA-Z0-9_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-30s\033[0m %s\n", $$1, $$2}'

-----------------------------: 

build-and-test: ## Build and run tests
	@sh scripts/build-and-test.sh

generate-types: ## Generate types
	@sh scripts/generate-types.sh

format: ## Format the code
	forc fmt && cargo fmt

-------Deployer Scripts-------:

deploy: ## Run the deployment script for core contracts (usage: make deploy NETWORK=<mainnet|testnet>)
	@forc build && cd deploy-scripts && NETWORK=$(NETWORK) SECRET=$(SECRET) cargo run deploy

add-asset: ## Run the script to add assets to the protocol (usage: make add-asset NETWORK=<mainnet|testnet> ASSET=ETH)
	@forc build && cd deploy-scripts && NETWORK=$(NETWORK) SECRET=$(SECRET) cargo run add-asset $(ASSET)

pause: ## Pause the protocol (usage: make pause NETWORK=<mainnet|testnet>)
	@cd deploy-scripts && NETWORK=$(NETWORK) SECRET=$(SECRET) cargo run pause

unpause: ## Unpause the protocol (usage: make unpause NETWORK=<mainnet|testnet>)
	@cd deploy-scripts && NETWORK=$(NETWORK) SECRET=$(SECRET) cargo run unpause

sanity-check: ## Run the sanity check script (usage: make sanity-check NETWORK=<mainnet|testnet>)
	@cd deploy-scripts && NETWORK=$(NETWORK) SECRET=$(SECRET) cargo run sanity-check

transfer-owner: ## Transfer ownership of the protocol (usage: make transfer-owner NETWORK=<mainnet|testnet> ADDRESS=<new_owner_address>)
	@cd deploy-scripts && NETWORK=$(NETWORK) SECRET=$(SECRET) cargo run transfer-owner $(ADDRESS)

initialize-hint-helper: ## Initialize the hint helper (usage: make initialize-hint-helper NETWORK=<mainnet|testnet>)
	@cd deploy-scripts && NETWORK=$(NETWORK) SECRET=$(SECRET) cargo run initialize-hint-helper

test-hint-helper: ## Test the hint helper (usage: make test-hint-helper NETWORK=<mainnet|testnet>)
	@cd deploy-scripts && NETWORK=$(NETWORK) SECRET=$(SECRET) cargo run test-hint-helper
	
migrate-v2: ## Migrate contracts to v2 (usage: make migrate-v2 NETWORK=<mainnet|testnet>)
	@forc build && cd deploy-scripts && NETWORK=$(NETWORK) SECRET=$(SECRET) cargo run migrate-v2
