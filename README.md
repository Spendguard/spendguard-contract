# SpendGuard Contract

A minimal, **read-only** Soroban helper contract that provides a stable,
versioned interface for querying an
[OpenZeppelin `stellar-accounts`](https://github.com/OpenZeppelin/stellar-contracts)
smart account's `spending_limit` policy state.

This is a component of the [SpendGuard](https://github.com/spendguard/spendguard)
monitoring and alerting layer for x402 agentic payments on Stellar.

## What it does

The `policy-view-helper` contract exposes two public functions:

| Function | Returns | Description |
|---|---|---|
| `get_version` | `u32` | WASM version marker for indexer compatibility checks. |
| `get_spending_limit_state(account, context_rule_id)` | `SpendingLimitView` | Reads the spending cap, rolling window, and current spent total from the OZ `spending_limit` policy installed on a specific context rule. |

### Cross-contract call flow

```
SpendGuard Indexer
       │
       ▼
policy-view-helper ──get_context_rule()──► OZ Smart Account
       │                                         │
       │                                         ▼
       │                                  ContextRule { policies: [...] }
       │
       └──get_spending_limit_data()──► spending_limit Policy Contract
                                               │
                                               ▼
                                        SpendingLimitData {
                                            spending_limit,
                                            period_ledgers,
                                            spending_history,
                                            cached_total_spent,
                                        }
```

## What it does NOT do

- ❌ No enforcement — it never blocks or reverses a transaction.
- ❌ No custody — it never holds or moves funds.
- ❌ No events — it never emits events (relies on the OZ contract's own events).
- ❌ No `simple_threshold` or `weighted_threshold` reads — only `spending_limit` (MVP scope).
- ❌ No support for non-transfer contexts — the OZ `spending_limit` policy only applies to `transfer` calls within `CallContract` context rules (see [OZ forum discussion, 2026-06-26]).

## Repo structure

```
spendguard-contract/
├── Cargo.toml                  # Workspace root
├── rust-toolchain.toml         # Pinned Rust toolchain (wasm32v1-none)
├── contracts/
│   └── policy-view-helper/
│       ├── Cargo.toml          # Contract crate manifest
│       ├── src/
│       │   ├── lib.rs          # Contract entry point (PolicyViewHelper impl)
│       │   ├── types.rs        # SpendingLimitView, PolicyViewError, OZ mirror types
│       │   └── read.rs         # get_spending_limit_state, get_version
│       └── src/test.rs         # Integration tests
├── CONTRIBUTING.md
├── SECURITY.md
└── README.md
```

## Tech stack

| Component | Version |
|---|---|
| Rust edition | 2021 |
| `soroban-sdk` | 27.0.3 (latest stable) |
| `stellar-accounts` | =0.7.1 (pinned) |
| WASM target | `wasm32v1-none` |
| Rust toolchain | stable (matching OZ) |

## Build & test

```bash
# Build the contract WASM
stellar contract build

# Run integration tests
cargo test --workspace
```

## Dependencies

The `policy-view-helper` depends on deployed OZ contracts at operator-configured
addresses. It does NOT store any addresses itself — everything is resolved from
on-chain data at query time via cross-contract calls.

## License

MIT
