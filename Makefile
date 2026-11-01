# policy-view-helper — build, verify and deploy.
#
# Every target is safe to run blind except `deploy`, which needs an explicit
# source account and refuses mainnet unless told twice. Run `make` for the list.
#
# Requirements: the toolchain pinned in rust-toolchain.toml (rustup installs it
# on first cargo invocation) and stellar-cli >= 25.2.0 (`stellar --version`).
# The CLI's build is the one that works here: a bare
# `cargo build --release --target wasm32v1-none` fails on this crate because the
# SDK's `experimental_spec_shaking_v2` feature is only driven correctly through
# `stellar contract build`, and `stellar contract optimize` is deprecated in 25+
# because `build` now optimizes by default.

# The Cargo package is `policy-view-helper`; the cdylib it builds is
# `policy_view_helper.wasm`, because cargo underscores the crate name in the
# artefact. Both names appear below, so they are stated once each.
PACKAGE      := policy-view-helper
ARTIFACT     := policy_view_helper
WASM         := target/wasm32v1-none/release/$(ARTIFACT).wasm
DEPLOYED_WASM := target/$(ARTIFACT).deployed.wasm

# Published instance — keep in step with the "Deployed instances" table in
# README.md. `make verify` reads both values back off the chain and says so if
# they are not.
CONTRACT_ID        ?= CCAM4NRAUB6SO3XLL2SRZQEHSOUYQGDKGNPCUIQ5KKI2S6QKWC2VN6NX
EXPECTED_WASM_HASH ?= ff125048e43d9e0f1139b3b89a348ac3916342cf9825b0fbc112d68b17735139

NETWORK           ?= testnet
SOROBAN_RPC_URL   ?= https://soroban-testnet.stellar.org
NETWORK_PASSPHRASE ?= Test SDF Network ; September 2015

STELLAR ?= stellar

# Name a key already in the CLI's own config (`stellar keys ls` to list,
# `stellar keys generate <name>` + `stellar keys fund <name>` for a new one) and
# pass it with SOURCE_ACCOUNT=<name>. The secret never enters this file, a
# target, an argument on the command line, or a commit.
SOURCE_ACCOUNT ?=

.PHONY: help build optimize test clippy check hash interface versions \
        fetch fetch-hash spec-deployed verify deploy clean

help:
	@echo "make build            compile to $(WASM) — the artefact you deploy"
	@echo "make optimize         same as build; stellar-cli 25+ optimizes in the build step"
	@echo "make test             cargo test --workspace"
	@echo "make clippy           cargo clippy --workspace --all-targets -- -D warnings (the CI gate)"
	@echo "make check            test + clippy"
	@echo "make hash             sha256 of the built WASM, which is the on-chain WASM id"
	@echo "make interface        the contract spec the built WASM publishes"
	@echo "make versions         stellar / rustc / cargo this build runs on"
	@echo "make fetch            download the deployed WASM to $(DEPLOYED_WASM)"
	@echo "make fetch-hash       sha256 of the deployed WASM"
	@echo "make spec-deployed    the deployed contract's spec, for diffing against make interface"
	@echo "make verify           build + fetch, then compare hashes and specs"
	@echo "make deploy SOURCE_ACCOUNT=<key-name>   deploy the built WASM to NETWORK=$(NETWORK)"
	@echo "make clean"

build:
	$(STELLAR) contract build --package $(PACKAGE)

# `contract optimize` is deprecated in stellar-cli 25+; build optimizes in the
# same step. Kept as a named target so the artefact path is spelled in one place.
optimize: build
	@echo "$(WASM) is already the optimized artefact (stellar contract build --optimize is on by default)"

test:
	cargo test --workspace

clippy:
	cargo clippy --workspace --all-targets -- -D warnings

check: test clippy

hash: build
	@sha256sum $(WASM)

interface: build
	@$(STELLAR) contract info interface --wasm $(WASM)

versions:
	@$(STELLAR) --version
	@rustc --version
	@cargo --version

fetch:
	@mkdir -p $(dir $(DEPLOYED_WASM))
	$(STELLAR) contract fetch --rpc-url $(SOROBAN_RPC_URL) \
		--network-passphrase "$(NETWORK_PASSPHRASE)" --contract-id $(CONTRACT_ID) \
		-o $(DEPLOYED_WASM)
	@echo "deployed WASM: $(DEPLOYED_WASM)"

fetch-hash: fetch
	@sha256sum $(DEPLOYED_WASM)

spec-deployed:
	@$(STELLAR) contract info interface --contract-id $(CONTRACT_ID) \
		--rpc-url $(SOROBAN_RPC_URL) --network-passphrase "$(NETWORK_PASSPHRASE)"

# Reproduces the published artefact against this checkout. A hash mismatch is
# not automatically a problem — the deployment may be a deliberately older,
# audited WASM — but it has to be explainable, and the spec diff is what
# explains it: identical specs mean the instance is the same public API built
# from other source, differing specs mean it predates this README.
verify: build fetch
	@built=$$(sha256sum $(WASM) | cut -d' ' -f1); \
	deployed=$$(sha256sum $(DEPLOYED_WASM) | cut -d' ' -f1); \
	echo "built       $$built"; \
	echo "deployed    $$deployed"; \
	echo "README says $(EXPECTED_WASM_HASH)"; \
	if [ -z "$$built" ] || [ -z "$$deployed" ]; then \
		echo "FAIL: no WASM to compare — 'make build' or 'make fetch' did not produce one"; exit 1; \
	fi; \
	if [ "$$deployed" != "$(EXPECTED_WASM_HASH)" ]; then \
		echo "FAIL: $(CONTRACT_ID) is not the WASM the README records"; exit 1; \
	fi; \
	if [ "$$built" = "$$deployed" ]; then \
		echo "MATCH: the published instance is this checkout"; \
	else \
		echo "DIFFERS: the published instance was built from other source — see README 'Deployed instances'"; \
	fi
	@$(STELLAR) contract info interface --wasm $(WASM) > target/spec.built.txt
	@$(STELLAR) contract info interface --contract-id $(CONTRACT_ID) \
		--rpc-url $(SOROBAN_RPC_URL) --network-passphrase "$(NETWORK_PASSPHRASE)" \
		> target/spec.deployed.txt
	@if [ ! -s target/spec.built.txt ] || [ ! -s target/spec.deployed.txt ]; then \
		echo "SPEC: unreadable — 'stellar contract info interface' returned no data for one of the two WASMs; that is not a match"; exit 1; \
	elif diff -q target/spec.built.txt target/spec.deployed.txt >/dev/null; then \
		echo "SPEC: identical — the deployed instance exposes this README's API"; \
	else \
		echo "SPEC: DIFFERENT — diff target/spec.built.txt against target/spec.deployed.txt; the deployed instance is not the contract this README documents"; \
	fi

# NETWORK is a variable, so the guard has to be explicit: a stray
# `make deploy NETWORK=public` must not be able to publish. Mainnet needs both
# NETWORK=public and ALLOW_MAINNET=1, and prints what it is about to do.
deploy: build
ifeq ($(strip $(SOURCE_ACCOUNT)),)
	@echo "make deploy needs SOURCE_ACCOUNT=<stellar key alias> (see 'stellar keys ls'). Refusing to guess an account."; exit 1
endif
	@if [ "$(NETWORK)" != "testnet" ] && [ "$(ALLOW_MAINNET)" != "1" ]; then \
		echo "Refusing to deploy to NETWORK=$(NETWORK) without ALLOW_MAINNET=1."; \
		echo "A read-only helper still becomes the address the indexer trusts; publish deliberately."; exit 1; \
	fi
	@if [ "$(NETWORK)" != "testnet" ]; then \
		echo "DEPLOYING TO MAINNET from $(WASM). 5s."; sleep 5; \
	fi
	$(STELLAR) contract deploy \
		--wasm $(WASM) \
		--network $(NETWORK) --source-account $(SOURCE_ACCOUNT) --auto-sign \
		--rpc-url $(SOROBAN_RPC_URL) --network-passphrase "$(NETWORK_PASSPHRASE)"

clean:
	cargo clean
