//! # Policy View Helper — Tests
//!
//! Integration tests using Soroban SDK test utilities and mock contracts
//! that match the OZ cross-contract interface.

use soroban_sdk::{
    contract, contractimpl, testutils::Address as _,
    Address, Env, String, Symbol, Vec,
};

use crate::types::{
    ContextRule, ContextRuleType,
    SpendingLimitData, SpendingEntry,
    SpendingLimitView,
};

// ──────────────────────────────────────────────────────────────────────────
// Mock Smart Account Contract
// ──────────────────────────────────────────────────────────────────────────

#[contract]
pub struct MockSmartAccount;

#[contractimpl]
impl MockSmartAccount {
    pub fn init(env: Env, rule: ContextRule) {
        env.storage().instance().set(&Symbol::new(&env, "rule"), &rule);
    }

    pub fn get_context_rule(env: Env, _context_rule_id: u32) -> ContextRule {
        env.storage()
            .instance()
            .get(&Symbol::new(&env, "rule"))
            .unwrap()
    }
}

// ──────────────────────────────────────────────────────────────────────────
// Mock Spending Limit Policy Contract
// ──────────────────────────────────────────────────────────────────────────

#[contract]
pub struct MockSpendingLimitPolicy;

#[contractimpl]
impl MockSpendingLimitPolicy {
    pub fn init(env: Env, data: SpendingLimitData) {
        env.storage()
            .instance()
            .set(&Symbol::new(&env, "data"), &data);
    }

    pub fn get_spending_limit_data(
        env: Env,
        _context_rule_id: u32,
        _smart_account: Address,
    ) -> SpendingLimitData {
        env.storage()
            .instance()
            .get(&Symbol::new(&env, "data"))
            .unwrap()
    }
}

// ──────────────────────────────────────────────────────────────────────────
// Test helpers
// ──────────────────────────────────────────────────────────────────────────

fn deploy_mock_smart_account(env: &Env, context_rule: &ContextRule) -> Address {
    let addr = env.register(MockSmartAccount, ());
    let client = MockSmartAccountClient::new(env, &addr);
    client.init(context_rule);
    addr
}

fn deploy_mock_spending_limit_policy(env: &Env, data: &SpendingLimitData) -> Address {
    let addr = env.register(MockSpendingLimitPolicy, ());
    let client = MockSpendingLimitPolicyClient::new(env, &addr);
    client.init(data);
    addr
}

fn default_spending_limit_data(env: &Env) -> SpendingLimitData {
    SpendingLimitData {
        spending_limit: 10_000_000_000,
        period_ledgers: 17280,
        spending_history: Vec::new(env),
        cached_total_spent: 2_500_000_000,
    }
}

fn default_context_rule(env: &Env, policy_addrs: Vec<Address>) -> ContextRule {
    ContextRule {
        id: 1,
        context_type: ContextRuleType::CallContract(Address::generate(env)),
        name: String::from_str(env, "test-rule"),
        valid_until: None,
        signers: Vec::new(env),
        policies: policy_addrs,
    }
}

// ──────────────────────────────────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────────────────────────────────

/// `get_version` always returns 1.
#[test]
fn test_get_version() {
    let env = Env::default();
    let contract_id = env.register(crate::PolicyViewHelper, ());
    let client = crate::PolicyViewHelperClient::new(&env, &contract_id);

    assert_eq!(client.get_version(), 1);
}

/// Positive case: spending limit policy installed with data.
#[test]
fn test_get_spending_limit_state_positive() {
    let env = Env::default();
    let policy_addr =
        deploy_mock_spending_limit_policy(&env, &default_spending_limit_data(&env));
    let rule = default_context_rule(&env, Vec::from_array(&env, [policy_addr]));
    let account_addr = deploy_mock_smart_account(&env, &rule);

    let contract_id = env.register(crate::PolicyViewHelper, ());
    let client = crate::PolicyViewHelperClient::new(&env, &contract_id);

    let view: SpendingLimitView = client.get_spending_limit_state(&account_addr, 1);

    assert_eq!(view.cap, 10_000_000_000);
    assert_eq!(view.spent_in_window, 2_500_000_000);
    assert_eq!(view.window_seconds, 86400);
}
