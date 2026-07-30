#![no_std]
mod read;
mod types;

use soroban_sdk::{contract, contractimpl, Env};
use types::SpendingLimitView;

const WASM_VERSION: u32 = 1;

/// A minimal read-only helper contract that provides a stable, versioned
/// interface for querying an OZ `stellar-accounts` smart account's
/// `spending_limit` policy state.
///
/// This contract does NOT hold any funds, enforce any rules, or emit any
/// events. It is a pure convenience read wrapper for the SpendGuard
/// indexer.
#[contract]
pub struct PolicyViewHelper;

#[contractimpl]
impl PolicyViewHelper {
    /// Returns the WASM version of this helper contract.
    pub fn get_version(_env: Env) -> u32 {
        WASM_VERSION
    }
}

#[cfg(test)]
mod test;
