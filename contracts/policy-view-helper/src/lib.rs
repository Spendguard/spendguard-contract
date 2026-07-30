#![no_std]
mod read;
mod types;

use soroban_sdk::{contract, contractimpl, Address, Env};
use types::{PolicyViewError, SpendingLimitView};

const WASM_VERSION: u32 = 1;

/// A minimal read-only helper contract that provides a stable, versioned
/// interface for querying an OZ `stellar-accounts` smart account's
/// `spending_limit` policy state.
///
/// This contract does NOT hold any funds, enforce any rules, or emit any
/// events. It is a pure convenience read wrapper for the SpendGuard
/// indexer, so the indexer doesn't need to hand-decode multiple raw
/// ledger entries per account per poll.
///
/// # Dependencies
///
/// - OZ `stellar-accounts` smart account contract (at the address passed
///   as `account` to each call).
/// - A deployed `spending_limit` policy contract attached to the
///   queried context rule.
///
/// Neither address is stored in this contract — everything is resolved
/// on-chain at query time via cross-contract calls.
#[contract]
pub struct PolicyViewHelper;

#[contractimpl]
impl PolicyViewHelper {
    /// Returns the WASM version of this helper contract. Bump this
    /// constant whenever the public API or the return types change in a
    /// way the indexer needs to be aware of.
    pub fn get_version(_env: Env) -> u32 {
        WASM_VERSION
    }

    /// Queries the on-chain `spending_limit` policy state for a given
    /// smart account and context rule.
    ///
    /// This function performs two cross-contract calls:
    /// 1. Calls `get_context_rule` on the smart account contract to
    ///    verify the rule exists and retrieve the policy contract addresses.
    /// 2. Calls `get_spending_limit_data` on the installed
    ///    `spending_limit` policy contract to read the current cap,
    ///    window, and spent counters.
    ///
    /// # Arguments
    ///
    /// * `account` — The OZ smart account contract address.
    /// * `context_rule_id` — The context rule ID whose spending_limit
    ///   policy to inspect.
    ///
    /// # Returns
    ///
    /// `SpendingLimitView` containing the current cap, window duration,
    /// amount spent in the current window, and when the window started.
    ///
    /// # Errors
    ///
    /// * [`PolicyViewError::NoSpendingLimitPolicyInstalled`] — No
    ///   `spending_limit` policy is installed on this context rule, or
    ///   the policy contract didn't respond with valid data.
    /// * [`PolicyViewError::AccountNotFound`] — The smart account
    ///   contract rejected the query (e.g. the context rule doesn't
    ///   exist).
    /// * [`PolicyViewError::UnsupportedContextType`] — The context rule
    ///   targets a non-transfer context, so a `spending_limit` policy
    ///   would never apply there (per OZ's own documented scope limit).
    pub fn get_spending_limit_state(
        env: Env,
        account: Address,
        context_rule_id: u32,
    ) -> Result<SpendingLimitView, PolicyViewError> {
        read::get_spending_limit_state(&env, &account, context_rule_id)
    }
}

#[cfg(test)]
mod test;
