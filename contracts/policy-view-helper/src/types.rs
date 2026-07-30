use soroban_sdk::{contracterror, contracttype, Address, Env, Vec};

// ---------------------------------------------------------------------------
// Public view types — returned by this contract's API
// ---------------------------------------------------------------------------

/// The decoded spending limit state returned to the indexer.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct SpendingLimitView {
    /// Maximum amount (in stroops) allowed within the window.
    pub cap: i128,
    /// The rolling window duration, expressed in wall-clock seconds as
    /// approximated from the on-chain `period_ledgers` field (one ledger
    /// ≈ 5 seconds on Stellar).
    pub window_seconds: u64,
    /// Total amount (in stroops) spent so far in the current rolling window.
    pub spent_in_window: i128,
    /// Approximate Unix timestamp (seconds since epoch) when the current
    /// rolling window started.
    pub window_started_at: u64,
}

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

/// Errors that the `policy-view-helper` contract may return.
#[contracterror]
#[derive(Copy, Clone, Debug, PartialEq)]
#[repr(u32)]
pub enum PolicyViewError {
    /// No `spending_limit` policy is installed on this context rule.
    NoSpendingLimitPolicyInstalled = 1,
    /// The smart account contract rejected the query.
    AccountNotFound = 2,
    /// The context rule targets a non-transfer context.
    UnsupportedContextType = 3,
}

// ---------------------------------------------------------------------------
// Mirror types for decoding OZ cross-contract call responses
// ---------------------------------------------------------------------------

/// Mirror of `stellar_accounts::policies::spending_limit::SpendingLimitData`.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct SpendingLimitData {
    pub spending_limit: i128,
    pub period_ledgers: u32,
    pub spending_history: Vec<SpendingEntry>,
    pub cached_total_spent: i128,
}

/// Mirror of `stellar_accounts::policies::spending_limit::SpendingEntry`.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct SpendingEntry {
    pub amount: i128,
    pub ledger_sequence: u32,
}

/// Mirror of `stellar_accounts::smart_account::ContextRule`.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct ContextRule {
    pub id: u32,
    pub context_type: ContextRuleType,
    pub name: soroban_sdk::String,
    pub valid_until: Option<u32>,
    pub signers: Vec<ContextSigner>,
    pub policies: Vec<Address>,
}

/// Mirror of `stellar_accounts::smart_account::ContextRuleType`.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub enum ContextRuleType {
    Default,
    CallContract(Address),
    CreateContract(soroban_sdk::BytesN<32>),
}

/// Mirror of `stellar_accounts::smart_account::Signer`.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub enum ContextSigner {
    Delegated(Address),
    External(Address, soroban_sdk::BytesN<32>),
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

impl SpendingLimitView {
    /// Converts on-chain `SpendingLimitData` into a wall-clock-oriented
    /// `SpendingLimitView`.
    pub fn from_spending_limit_data(
        data: &SpendingLimitData,
        current_ledger: u32,
        approximate_ledger_close_time: u64,
    ) -> Self {
        let window_seconds = (data.period_ledgers as u64) * 5;

        let window_started_at = if let Some(entry) = data.spending_history.first() {
            let ledgers_ago = current_ledger.saturating_sub(entry.ledger_sequence);
            let seconds_ago = (ledgers_ago as u64) * 5;
            approximate_ledger_close_time.saturating_sub(seconds_ago)
        } else {
            approximate_ledger_close_time.saturating_sub(window_seconds)
        };

        SpendingLimitView {
            cap: data.spending_limit,
            window_seconds,
            spent_in_window: data.cached_total_spent,
            window_started_at,
        }
    }
}
