<div align="center">

# 🛡️ SpendGuard Contract

### Read-only Soroban `policy-view-helper` for OpenZeppelin smart-account spending limits

**The on-chain read half of SpendGuard. Zero custody. Zero writes. Zero signing.**

[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![CI](https://github.com/Spendguard/spendguard-contract/actions/workflows/ci.yml/badge.svg)](https://github.com/Spendguard/spendguard-contract/actions/workflows/ci.yml)
[![Rust](https://img.shields.io/badge/rust-stable-000000?logo=rust&logoColor=white)](rust-toolchain.toml)
[![Soroban SDK](https://img.shields.io/badge/soroban--sdk-26.1.0-7c3aed?logo=stellar&logoColor=white)](Cargo.toml)
[![Tests](https://img.shields.io/badge/tests-9%20passing-10b981?logo=rust&logoColor=white)](https://github.com/Spendguard/spendguard-contract/actions/workflows/ci.yml)
[![Stellar & Soroban](https://img.shields.io/badge/built%20for-Stellar%20%26%20Soroban-7c3aed?logo=stellar&logoColor=white)](#how-it-works)

</div>

`policy-view-helper` is the smart-contract half of the SpendGuard ecosystem: a minimal, **read-only** Soroban contract that gives the [SpendGuard indexer](https://github.com/Spendguard/spendguard-app) a single clean, versioned interface for querying an [OpenZeppelin `stellar-accounts`](https://github.com/OpenZeppelin/stellar-contracts) smart account's declared `spending_limit` policy state — no hand-decoding of nested ledger entries, no stale caches, no wall-clock guesswork.

It **never holds funds, never writes state, and never sits in the payment path** — it only answers questions about public on-chain policy state.

---

## TL;DR

| `policy-view-helper` **is** | `policy-view-helper` **never** |
|---|---|
| ✅ A read-only on-chain query wrapper | ❌ Holds, moves, or signs for funds |
| ✅ A versioned public interface (`WASM_VERSION = 2`) | ❌ Writes storage or emits events |
| ✅ Two cross-contract reads per query | ❌ Blocks, freezes, or enforces anything |
| ✅ Raw ledger-accurate policy values | ❌ Converts ledgers to wall-clock on-chain |

If the helper is wrong or down, **no funds are at risk** — the real backstop is the on-chain policy enforcement it merely reflects.

---

## Why this matters

Smart accounts built on OpenZeppelin's framework declare spending-limit guardrails on-chain. But reading that state requires traversing complex nested structures (`ContextRule` → installed `Policy` contracts → `SpendingLimitData`) and the indexer would have to hand-decode multiple raw ledger entries per account per poll cycle.

`policy-view-helper` collapses that into one call:

- 📖 **Three functions, one purpose** — a version probe, the policy-state read, and the per-policy enumeration that read needs when a rule carries more than one limit.
- 🔗 **Cross-contract reads, zero state** — every query resolves addresses on-chain at call time; nothing is stored, cached, or remembered.
- 🧮 **Raw on-chain values** — returns precise stroop amounts and ledger sequences; wall-clock conversion is left to the indexer, which has access to Soroban RPC close-time data.
- 🔒 **Versioned for indexers** — `get_version` lets the [SpendGuard indexer](https://github.com/Spendguard/spendguard-app) fail loudly instead of silently mis-reading a changed API.

---

## How it works

```mermaid
sequenceDiagram
    autonumber
    participant IX as SpendGuard Indexer / SDK
    participant VH as policy-view-helper (this contract)
    participant SA as OZ Smart Account
    participant PL as spending_limit Policy Contract

    IX->>VH: simulateContract: get_spending_limit_state(account, rule_id)
    VH->>SA: get_context_rule(rule_id)
    SA-->>VH: ContextRule { context_type, policies: [...] }
    VH->>VH: Reject rules whose context_type is not CallContract (Default, CreateContract)
    loop For each policy address in rule
        VH->>PL: get_spending_limit_data(rule_id, account)
        alt Policy returns valid SpendingLimitData
            PL-->>VH: SpendingLimitData { limit, period, history, spent }
            VH->>VH: Collect (policy, SpendingLimitView) — raw ledger-accurate values
        end
    end
    alt Exactly one policy answered
        VH-->>IX: SpendingLimitView
    else More than one answered
        VH-->>IX: PolicyViewError::MultipleSpendingLimitPolicies
    else None answered
        VH-->>IX: PolicyViewError::NoSpendingLimitPolicyInstalled
    end
```

### Public interface

| Function | Signature | Returns |
|---|---|---|
| `get_version` | `() -> u32` | `2` — WASM version marker for indexer compatibility checks |
| `get_spending_limit_state` | `(account: Address, context_rule_id: u32)` | `Result<SpendingLimitView, PolicyViewError>` |
| `get_spending_limit_policy_views` | `(account: Address, context_rule_id: u32)` | `Result<Vec<SpendingLimitPolicyView>, PolicyViewError>` |

### More than one `spending_limit` policy on a rule

An OZ context rule can carry several policies, and nothing stops two of them
being `spending_limit` with different caps. Until version 2 the helper returned
the **first policy that answered**, which is the account's storage order rather
than anything about the limits: with caps of 1 000 and 100 000 the dashboard
could render the loose one at 40 % used while the tight one was the limit that
actually blocked the next spend.

Version 2 splits the two questions:

- `get_spending_limit_policy_views` enumerates every responding policy, each
  labelled with the contract that reported it, so the caller can compute
  utilisation per limit and surface the worst.
- `get_spending_limit_state` keeps its shape — one `SpendingLimitView`, same
  fields — and answers `MultipleSpendingLimitPolicies` instead of choosing.
  Which limit "binds" depends on what the caller is asking (smallest remaining
  headroom and smallest cap disagree once the windows differ), and a read helper
  that picks silently is how a wrong number reaches a treasury dashboard.

Enumeration order follows the rule's own `policies` vector and is **not** a
ranking; every entry carries its address, so the order never has to be trusted
to be meaningful.

### `SpendingLimitView`

```rust
#[contracttype]
pub struct SpendingLimitView {
    /// Maximum amount (in stroops, 1 XLM = 10^7 stroops) in the rolling window.
    pub cap: i128,
    /// Rolling window length in ledgers (~5 s per ledger on Stellar).
    pub window_ledgers: u32,
    /// Total amount spent so far in the current rolling window.
    pub spent_in_window: i128,
    /// Ledger sequence of the earliest retained spending entry (0 = no history).
    pub window_started_ledger: u32,
}
```

### `SpendingLimitPolicyView`

```rust
#[contracttype]
pub struct SpendingLimitPolicyView {
    /// The policy contract that answered `get_spending_limit_data`.
    pub policy: Address,
    /// The state that policy reports.
    pub view: SpendingLimitView,
}
```

A struct rather than an `(Address, SpendingLimitView)` tuple, as #1's scope
suggested: the deployed spec then names both halves and a reader decodes by key
instead of by position. Decoding by position is how the app ended up looking for
`window_seconds` on a contract that emitted `window_ledgers`.

`window_started_ledger` is the **minimum** `ledger_sequence` in the policy's
spending history, not its first element. `stellar-accounts` 0.7.2 keeps that
vector oldest-first, so the two agree today; reading the minimum means the value
stays correct if the pinned dependency ever changes its storage layout.
`window_started_ledger_comes_from_the_real_policy_history` installs and enforces
through the real OZ policy (dev-dependency) to pin the ordering, and
`window_started_ledger_ignores_history_order` feeds an unsorted history through
the mirror types to pin the derivation.

### Errors

| Code | Error | Meaning |
|---|---|---|
| `1` | `NoSpendingLimitPolicyInstalled` | No policy on the rule answered `get_spending_limit_data` |
| `2` | `AccountNotFound` | The smart account rejected the query (rule missing, wrong address) |
| `3` | `UnsupportedContextType` | The context rule targets a non-transfer context, where `spending_limit` never applies |
| `4` | `MultipleSpendingLimitPolicies` | More than one policy answered, so no single view describes the limit — only `get_spending_limit_state` returns this |

Every row is named by a test. `policy_view_error(..)` in `test.rs` unwraps the four
layers of the generated `try_` client's result, so a test fails when the *wrong*
variant comes back — not only when the call fails — which is the distinction that
let #1 stay green behind `assert!(result.is_err())`.

---

## Security posture

SpendGuard's whole design is built on a single principle: **the least powerful tool that solves the problem.** This contract is the extreme version of that.

- 🔒 **Read-only by construction** — zero storage writes, zero state mutations, zero events emitted (verifiable by reading the source)
- 🔑 **Zero custody** — never holds, moves, or signs for funds
- 👀 **Public data only** — every input and output is public on-chain state; no secrets anywhere in the execution path
- 🧮 **No floats, no `unwrap` in production** — `i128` integer math throughout, all errors propagated explicitly
- 📖 See [SECURITY.md](./SECURITY.md) for the full scope, audit status, and disclosure process

---

## Quick start

```bash
git clone https://github.com/Spendguard/spendguard-contract.git
cd spendguard-contract
stellar contract build     # → target/wasm32v1-none/release/policy_view_helper.wasm
cargo test --workspace     # 20 tests: behaviour, OZ serialization-compat,
                             # window-start ordering, multi-policy
```

**Requirements:** Rust `stable` (pinned in `rust-toolchain.toml` with the `wasm32v1-none` target and `rustfmt`/`clippy`/`rust-src` components) and the [Soroban CLI](https://soroban.stellar.org/docs/cli).

### Deployed instances

| Network | Contract ID | `get_version` | Status |
|---|---|---|---|
| **Stellar Testnet** | `CCAM4NRAUB6SO3XLL2SRZQEHSOUYQGDKGNPCUIQ5KKI2S6QKWC2VN6NX` | `1` | Active |

This is the ID the [SpendGuard app](https://github.com/Spendguard/spendguard-app) reads via `POLICY_VIEW_HELPER_CONTRACT_ID`.

The published instance is **version 1** — it was deployed before #5, so it does
not have `get_spending_limit_policy_views` and its `get_spending_limit_state`
still returns the first policy that answered. `get_version` on the deployed
contract returns `1`, which is the discrepancy the app's own version probe
exists to catch; publishing version 2 needs a redeploy (see #7, no documented
build/optimize/deploy path yet).

---

## Architecture

```
spendguard-contract/
├── contracts/policy-view-helper/   # The single Soroban contract
│   ├── src/
│   │   ├── lib.rs                  # #[contract] entry point + public interface
│   │   ├── read.rs                 # Cross-contract query implementation
│   │   ├── types.rs                # SpendingLimitView, errors, OZ mirror types
│   │   └── test.rs                 # 20 tests: real-OZ round-trips, one
│   │                               #   specific PolicyViewError per failure
│   │                               #   mode, window-order driven by the policy
│   └── test_snapshots/             # Soroban snapshot fixtures
├── .github/workflows/ci.yml        # Test & Clippy on every push/PR to master
├── rust-toolchain.toml             # Pinned Rust stable + wasm32v1-none target
├── CONTRIBUTING.md · SECURITY.md · drips.json · LICENSE
└── Cargo.toml                      # Workspace root manifest
```

### Technical stack

| Component | Version / Target |
|---|---|
| Rust Edition | 2021 |
| Soroban SDK | `26.1.0` (`experimental_spec_shaking_v2`) |
| OpenZeppelin `stellar-accounts` | `=0.7.2` (pinned dev-dependency) |
| Soroban Test Helpers | `0.2.4` |
| Compilation Target | `wasm32v1-none` |
| Release profile | `opt-level = "z"`, `lto = true`, `overflow-checks = true`, `panic = "abort"` |

The `stellar-accounts` dependency is **pinned** and only used in tests — the production contract carries its own mirror types and verifies they still decode real OZ data via `oz_serialization_compat` round-trip tests.

---

## Scripts & CI

| Command | Description |
|---|---|
| `stellar contract build` | Builds the optimized WASM to `target/wasm32v1-none/release/policy_view_helper.wasm` |
| `cargo test --workspace` | Runs the 20-test suite (positive, negative — each asserting a specific `PolicyViewError` — OZ compatibility, window-start ordering, and multi-policy cases) |
| `cargo clippy --workspace --all-targets -- -D warnings` | Lints with warnings denied (CI gate) |

CI (GitHub Actions) runs **Test & Clippy** on every push and pull request to `master`.

---

## Known limitations (honest)

1. **`spending_limit` policy type only** — the helper reads the `spending_limit` policy installed on a transfer-context rule. `simple_threshold` and `weighted_threshold` policies are not queried (yet), and OZ scopes `spending_limit` to transfer contexts by design — an upstream constraint, not a SpendGuard one.
2. **Ledger windows, not wall-clock** — all window fields are ledger sequences; the indexer is responsible for converting them to timestamps using Soroban RPC close-time data.
3. **Mirror types must track OpenZeppelin** — the production mirror types are verified against `stellar-accounts` 0.7.2; an OZ upgrade requires re-running the compatibility tests and the real-policy window-order test (which asserts the retained history order directly) before the pin moves.
4. **No events emitted** — indexing still relies on the OZ contracts' own `transfer` / `spending_limit_enforced` events; this contract is query-only.

---

## Roadmap

- 🧩 Support additional OZ policy types (`simple_threshold`, `weighted_threshold`)
- 🌐 Mainnet deployment alongside the app's live-testnet flow
- ⚡ Batch / multi-rule convenience queries for the indexer
- 🔐 Independent audit of the narrow cross-contract read path

---

## Changelog

### `WASM_VERSION = 2` — multi-policy rules (#5)

Breaking for a rule that carries more than one `spending_limit` policy; a rule
with one policy behaves exactly as before.

| Change | What the indexer must do |
|---|---|
| `get_spending_limit_state` returns `MultipleSpendingLimitPolicies` (`#4`) instead of the first answering policy | Treat `#4` as *ambiguous*, never as *no policy*. On the deployed v1 contract it cannot appear; on v2 a caller that maps every `Error(Contract, #N)` to "nothing to show" would render an account with two limits as having none |
| New `get_spending_limit_policy_views` → `Vec<SpendingLimitPolicyView>` | Use it to compute utilisation per policy and surface the tightest; entries carry `policy: Address` so the enumeration order never has to be trusted |
| `SpendingLimitView` fields unchanged (`cap`, `window_ledgers`, `spent_in_window`, `window_started_ledger`) | Existing decoding keeps working |
| `get_version` → `2` | Probe it before reading, and fail loudly on a version the decoder has not been checked against |

### `WASM_VERSION = 1`

`get_spending_limit_state` + `get_version`, reading one `spending_limit` policy
per context rule. The window fields have been ledger-native since commit
`b4bd728`, which removed the invented 5 s-per-ledger conversion the first draft
returned.

---

## Contributing

Contributors are welcome — and rewarded. This contract is a great **Drips-wave contribution target**: ~300 lines of Rust, a pinned dependency, and a test suite that encodes the entire contract surface.

- 🎯 Start with the issues labelled **good first issue**
- 📖 Read [CONTRIBUTING.md](./CONTRIBUTING.md) — Conventional Commits, no `unwrap` outside tests, positive + negative test required per function
- 🐛 Security-sensitive bugs go through [SECURITY.md](./SECURITY.md)
- 🧪 `cargo test --workspace` before opening a PR

---

## License

[MIT](LICENSE) © SpendGuard Contributors
