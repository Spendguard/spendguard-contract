#![no_std]
mod read;
mod types;

use soroban_sdk::contract;

/// A minimal read-only helper contract that provides a stable, versioned
/// interface for querying an OZ `stellar-accounts` smart account's
/// `spending_limit` policy state.
///
/// This contract does NOT hold any funds, enforce any rules, or emit any
/// events. It is a pure convenience read wrapper for the SpendGuard
/// indexer.
#[contract]
pub struct PolicyViewHelper;

#[cfg(test)]
mod test;
