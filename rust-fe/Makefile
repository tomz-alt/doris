SHELL := /bin/bash

.PHONY: all build test mysql-cli-integration mysql-cli-integration-no-start

all: test

build:
	cargo build

test:
	cargo test

# Run MySQL CLI integration with the FE lifecycle managed by the script.
# This will:
#   - build the debug FE binary
#   - start it on an ephemeral port
#   - run SELECT 1 / SHOW DATABASES / SHOW TABLES via mysql CLI
#   - stop the FE on completion
mysql-cli-integration: build
	FE_BIN=./target/debug/doris-rust-fe scripts/mysql_cli_integration.sh --start-fe

# Run MySQL CLI integration against an already-running FE.
# Assumes FE_QUERY_PORT (or default 9030) is reachable.
mysql-cli-integration-no-start:
	scripts/mysql_cli_integration.sh

