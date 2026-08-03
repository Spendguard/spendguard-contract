# SpendGuard Contract

**Read-only Soroban Helper Contract for Querying OpenZeppelin Smart Account Policy State on Stellar.**

The `spendguard-contract` repository contains `policy-view-helper` — a minimal, zero-custody, read-only Soroban smart contract. It provides a stable, versioned interface for querying an [OpenZeppelin `stellar-accounts`](https://github.com/OpenZeppelin/stellar-contracts) smart account's declared `spending_limit` policy state directly on-chain.

This repository serves as the smart contract component of the [SpendGuard](https://github.com/Spendguard/spendguard-app) monitoring, auditing, and alerting platform for x402 payments on Stellar.

---

## 1. Executive Summary & Capabilities

Soroban smart accounts using OpenZeppelin's framework declare spending limit guardrails on-chain. However, querying policy state directly requires traversing complex nested data structures (`ContextRule`, `Policy`, `SpendingLimitData`).

The `policy-view-helper` contract simplifies off-chain client queries by executing on-chain cross-contract reads and returning a single, clean `SpendingLimitView` struct.

### Public Contract Interface

| Function | Signature | Return Type | Description |
|---|---|---|---|
| `get_version` | `() -> u32` | `u32` | Returns the WASM contract version marker (e.g. `1`) for indexer compatibility validation. |
| `get_spending_limit_state` | `(account: Address, context_rule_id: u32)` | `SpendingLimitView` | Cross-queries the target smart account and its policy contract, returning raw on-chain cap, period ledgers, spent total, and window start ledger. |

---

## 2. On-Chain Cross-Contract Call Architecture

When `get_spending_limit_state` is invoked, `policy-view-helper` performs cross-contract reads across the smart account and policy contracts:

```
┌─────────────────────────┐
│   SpendGuard Indexer /  │
│   SDK (Simulate Call)   │
└────────────┬────────────┘
             │
             │ get_spending_limit_state(account, context_rule_id)
             ▼
┌─────────────────────────┐
│   policy-view-helper    │
└────────────┬────────────┘
             │
             ├─── 1. get_context_rule(context_rule_id) ───► ┌─────────────────────────┐
             │                                              │ OpenZeppelin Smart      │
             │ ◄── Returns ContextRule { policies: [...] } ──┤ Account (Address)       │
             │                                              └─────────────────────────┘
             │
             └─── 2. get_spending_limit_data(account) ───► ┌─────────────────────────┐
                                                            │ spending_limit Policy   │
                                                            │ Contract                │
                                                            └────────────┬────────────┘
                                                                         │
                                                                         ▼
                                                            Returns SpendingLimitData {
                                                                spending_limit: i128,
                                                                period_ledgers: u64,
                                                                spending_history: Map,
                                                                cached_total_spent: i128,
                                                            }
```

---

## 3. Data Structures

### `SpendingLimitView`

```rust
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SpendingLimitView {
    /// Raw spending cap in stroops (1 XLM = 10^7 stroops).
    pub cap: i128,
    /// Policy window duration in ledgers (1 ledger ≈ 5 seconds).
    pub window_ledgers: u64,
    /// Total amount spent within the current active window.
    pub spent_in_window: i128,
    /// Ledger sequence number when the current window started.
    pub window_started_ledger: u64,
}
```

---

## 4. Design Guarantees & Non-Goals

- ✅ **Read-Only Guarantee:** The contract contains zero state mutations, zero storage writes, and zero state modifications.
- ✅ **Zero Custody & Zero Funds Risk:** It never holds, moves, or signs for funds.
- ✅ **Raw On-Chain Ledger Values:** Returns precise ledger sequence numbers without performing lossy wall-clock conversions on-chain.
- ❌ **No Transaction Blocking:** It never sits in the execution path and cannot block payments.
- ❌ **No Events Emitted:** Relies strictly on OpenZeppelin contract events for indexing.

---

## 5. Repository Structure

```
spendguard-contract/
├── Cargo.toml                  # Workspace root manifest
├── rust-toolchain.toml         # Pinned Rust toolchain (wasm32v1-none)
├── contracts/
│   └── policy-view-helper/
│       ├── Cargo.toml          # Contract crate manifest
│       └── src/
│           ├── lib.rs          # Contract interface & entry point
│           ├── types.rs        # SpendingLimitView, PolicyViewError, OZ mirror types
│           ├── read.rs         # Function implementations
│           └── test.rs         # Soroban integration test suite
├── CONTRIBUTING.md
├── SECURITY.md
└── README.md
```

---

## 6. Technical Stack & Pinned Versions

| Component | Version / Target |
|---|---|
| Rust Edition | 2021 |
| Soroban SDK | `26.1.0` (with `experimental_spec_shaking_v2`) |
| OpenZeppelin `stellar-accounts` | `0.7.2` (pinned) |
| Compilation Target | `wasm32v1-none` |

---

## 7. Build, Test & Deployment

### Build WASM Bytecode

```bash
# Build optimized webassembly contract binary
stellar contract build
```

The compiled output will be generated at `target/wasm32v1-none/release/policy_view_helper.wasm`.

### Run Test Suite

```bash
# Run unit and integration tests using Soroban environment mocks
cargo test --workspace
```

### Deployed Network Instances

| Network | Contract ID | Status |
|---|---|---|
| **Stellar Testnet** | `CCAM4NRAUB6SO3XLL2SRZQEHSOUYQGDKGNPCUIQ5KKI2S6QKWC2VN6NX` | Active |

---

## 8. License

[MIT](LICENSE) © SpendGuard Contributors
