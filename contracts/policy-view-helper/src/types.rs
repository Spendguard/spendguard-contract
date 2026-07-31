use soroban_sdk::{contracterror, contracttype, Address, Vec};

// ---------------------------------------------------------------------------
// Public view types — returned by this contract's API
// ---------------------------------------------------------------------------

/// The decoded spending limit state returned to the indexer.
///
/// All fields map directly from the on-chain `SpendingLimitData` stored by
/// the OZ `spending_limit` policy contract.  No wall-clock conversion is
/// performed — the indexer should convert ledger-sequence-based fields to
/// timestamps using precise Soroban RPC close-time data.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct SpendingLimitView {
    /// Maximum amount (in stroops) allowed within the rolling window.
    pub cap: i128,
    /// The rolling window length in ledgers (raw `period_ledgers` from
    /// the OZ policy).  The indexer should multiply by the average
    /// ledger-close time (~5 s on Stellar mainnet, but variable) to
    /// estimate wall-clock duration, or use actual close-time data from
    /// Soroban RPC `getLedgerEntries` for precision.
    pub window_ledgers: u32,
    /// Total amount (in stroops) spent so far in the current rolling
    /// window (raw `cached_total_spent` from the OZ policy).
    pub spent_in_window: i128,
    /// The ledger sequence of the earliest still-retained spending entry,
    /// or `0` if the window has no spending history yet.  The indexer
    /// can look up this ledger's close time via Soroban RPC to
    /// determine the precise window start time.
    pub window_started_ledger: u32,
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
    pub signers: Vec<ContextSigner>,
    pub signer_ids: Vec<u32>,
    pub policies: Vec<Address>,
    pub policy_ids: Vec<u32>,
    pub valid_until: Option<u32>,
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
    External(Address, soroban_sdk::Bytes),
}

// ---------------------------------------------------------------------------
// Conversions: OZ data → view
// ---------------------------------------------------------------------------

impl From<SpendingLimitData> for SpendingLimitView {
    /// Maps the on-chain `SpendingLimitData` directly to a
    /// `SpendingLimitView`, preserving all raw OZ values without
    /// wall-clock conversion.
    fn from(data: SpendingLimitData) -> Self {
        let window_started_ledger = data
            .spending_history
            .first()
            .map(|entry| entry.ledger_sequence)
            .unwrap_or(0);

        SpendingLimitView {
            cap: data.spending_limit,
            window_ledgers: data.period_ledgers,
            spent_in_window: data.cached_total_spent,
            window_started_ledger,
        }
    }
}
