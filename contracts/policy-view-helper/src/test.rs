//! # Policy View Helper — Tests

use soroban_sdk::{
    contract, contractimpl, testutils::Address as _,
    Address, BytesN, Env, IntoVal, String, Symbol, TryFromVal, Val, Vec,
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
// Mock Policy (Not Spending-Limit)
// ──────────────────────────────────────────────────────────────────────────

#[contract]
pub struct MockOtherPolicy;

#[contractimpl]
impl MockOtherPolicy {
    pub fn some_other_function(_env: Env) -> u32 {
        42
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

fn deploy_mock_other_policy(env: &Env) -> Address {
    env.register(MockOtherPolicy, ())
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
    assert_eq!(view.window_ledgers, 17280);
    assert_eq!(view.window_started_ledger, 0);
}

/// Negative case: no policy installed on the context rule.
#[test]
fn test_get_spending_limit_state_no_policy() {
    let env = Env::default();
    let rule = default_context_rule(&env, Vec::new(&env));
    let account_addr = deploy_mock_smart_account(&env, &rule);

    let contract_id = env.register(crate::PolicyViewHelper, ());
    let client = crate::PolicyViewHelperClient::new(&env, &contract_id);

    let result = client.try_get_spending_limit_state(&account_addr, 1);
    assert!(result.is_err(), "expected error but got Ok");
}

/// Negative case: policies exist but none expose
/// `get_spending_limit_data`.
#[test]
fn test_get_spending_limit_state_no_matching_policy() {
    let env = Env::default();
    let other_policy = deploy_mock_other_policy(&env);
    let rule = default_context_rule(&env, Vec::from_array(&env, [other_policy]));
    let account_addr = deploy_mock_smart_account(&env, &rule);

    let contract_id = env.register(crate::PolicyViewHelper, ());
    let client = crate::PolicyViewHelperClient::new(&env, &contract_id);

    let result = client.try_get_spending_limit_state(&account_addr, 1);
    assert!(result.is_err(), "expected error but got Ok");
}

/// Negative case: context rule type is `CreateContract`.
#[test]
fn test_get_spending_limit_state_unsupported_context() {
    let env = Env::default();
    let policy_addr =
        deploy_mock_spending_limit_policy(&env, &default_spending_limit_data(&env));

    let create_contract_rule = ContextRule {
        id: 2,
        context_type: ContextRuleType::CreateContract(BytesN::from_array(
            &env,
            &[0u8; 32],
        )),
        name: String::from_str(&env, "create-rule"),
        valid_until: None,
        signers: Vec::new(&env),
        policies: Vec::from_array(&env, [policy_addr]),
    };
    let account_addr = deploy_mock_smart_account(&env, &create_contract_rule);

    let contract_id = env.register(crate::PolicyViewHelper, ());
    let client = crate::PolicyViewHelperClient::new(&env, &contract_id);

    let result = client.try_get_spending_limit_state(&account_addr, 2);
    assert!(result.is_err(), "expected error but got Ok");
}

/// Edge case: spending history has entries → `window_started_ledger`
/// matches the earliest entry's ledger sequence.
#[test]
fn test_get_spending_limit_state_with_history() {
    let env = Env::default();
    let current_ledger = env.ledger().sequence();

    let mut data = default_spending_limit_data(&env);
    let entry1 = SpendingEntry {
        amount: 1_000_000_000,
        ledger_sequence: current_ledger - 1000,
    };
    let entry2 = SpendingEntry {
        amount: 1_500_000_000,
        ledger_sequence: current_ledger - 500,
    };
    data.spending_history = Vec::from_array(&env, [entry1, entry2]);
    data.cached_total_spent = 2_500_000_000;

    let policy_addr = deploy_mock_spending_limit_policy(&env, &data);
    let rule = default_context_rule(&env, Vec::from_array(&env, [policy_addr]));
    let account_addr = deploy_mock_smart_account(&env, &rule);

    let contract_id = env.register(crate::PolicyViewHelper, ());
    let client = crate::PolicyViewHelperClient::new(&env, &contract_id);

    let view: SpendingLimitView = client.get_spending_limit_state(&account_addr, 1);

    assert_eq!(view.spent_in_window, 2_500_000_000);
    // window_started_ledger should equal the earliest entry's ledger sequence
    assert_eq!(view.window_started_ledger, current_ledger - 1000);
}

// ──────────────────────────────────────────────────────────────────────────
// OZ Serialization Compatibility Tests
//
// These tests verify that our mirror types (defined in `types.rs`) can
// correctly decode data serialized by the real `stellar-accounts` v0.7.1
// types.  This catches field-order or memory-layout mismatches between
// the OZ `#[contracttype]` structs and our local mirrors.
//
// Because `stellar-accounts` is a dev-dependency, we import the real
// types directly and round-trip through `Env`-based serialization.
// ──────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod oz_serialization_compat {
    use soroban_sdk::{testutils::Address as _, Address, Env, IntoVal, TryFromVal, Val, Vec};

    // Import the REAL stellar-accounts v0.7.1 types, aliased to avoid name
    // collisions with our mirror types.
    use stellar_accounts::policies::spending_limit::{
        SpendingLimitData as RealData,
        SpendingEntry as RealEntry,
    };
    use stellar_accounts::smart_account::{
        ContextRule as RealContextRule,
        ContextRuleType as RealContextRuleType,
    };

    use crate::types::{
        ContextRule as OurContextRule,
        ContextRuleType as OurContextRuleType,
        SpendingLimitData as OurData,
        SpendingEntry as OurEntry,
    };

    /// Verifies that our `SpendingLimitData` mirror can decode data
    /// serialized by the real OZ `SpendingLimitData`.
    #[test]
    fn test_mirror_decodes_spending_limit_data() {
        let env = Env::default();

        let real_entry = RealEntry {
            amount: 1_000_000_000i128,
            ledger_sequence: 100_000_u32,
        };
        let real_data = RealData {
            spending_limit: 10_000_000_000i128,
            period_ledgers: 17280_u32,
            spending_history: Vec::from_array(&env, [real_entry]),
            cached_total_spent: 1_000_000_000i128,
        };

        // Serialize using the real OZ type
        let val: Val = real_data.into_val(&env);

        // Deserialize using our mirror type
        let our_data: OurData = match OurData::try_from_val(&env, &val) {
            Ok(d) => d,
            Err(()) => panic!("Our SpendingLimitData mirror failed to decode real OZ data"),
        };

        // Verify all fields match
        assert_eq!(our_data.spending_limit, real_data.spending_limit);
        assert_eq!(our_data.period_ledgers, real_data.period_ledgers);
        assert_eq!(our_data.cached_total_spent, real_data.cached_total_spent);
        assert_eq!(our_data.spending_history.len(), 1);

        let our_entry = our_data.spending_history.get(0).unwrap();
        assert_eq!(our_entry.amount, real_entry.amount);
        assert_eq!(our_entry.ledger_sequence, real_entry.ledger_sequence);
    }

    /// Verifies that our `ContextRule` mirror can decode data serialized
    /// by the real OZ `ContextRule`.
    #[test]
    fn test_mirror_decodes_context_rule() {
        let env = Env::default();

        let policy_addr = Address::generate(&env);

        let real_rule = RealContextRule {
            id: 42_u32,
            context_type: RealContextRuleType::CallContract(Address::generate(&env)),
            name: soroban_sdk::String::from_str(&env, "test-rule"),
            valid_until: Some(200_000_u32),
            signers: Vec::new(&env),
            policies: Vec::from_array(&env, [policy_addr]),
        };

        // Serialize using the real OZ type
        let val: Val = real_rule.into_val(&env);

        // Deserialize using our mirror type
        let our_rule: OurContextRule = match OurContextRule::try_from_val(&env, &val) {
            Ok(r) => r,
            Err(()) => panic!("Our ContextRule mirror failed to decode real OZ ContextRule"),
        };

        // Verify fields match
        assert_eq!(our_rule.id, real_rule.id);
        assert_eq!(our_rule.name, real_rule.name);
        assert_eq!(our_rule.valid_until, real_rule.valid_until);
        assert_eq!(our_rule.policies.len(), 1);
        assert_eq!(our_rule.policies.get(0).unwrap(), policy_addr);

        // Verify context type round-trips
        match (&our_rule.context_type, &real_rule.context_type) {
            (
                OurContextRuleType::CallContract(our_addr),
                RealContextRuleType::CallContract(real_addr),
            ) => assert_eq!(our_addr, real_addr),
            _ => panic!("ContextRuleType variant mismatch after round-trip"),
        }
    }

    /// Verifies that our `SpendingEntry` mirror can decode data
    /// serialized by the real OZ `SpendingEntry`.
    #[test]
    fn test_mirror_decodes_spending_entry() {
        let env = Env::default();

        let real_entry = RealEntry {
            amount: 5_000_000_000i128,
            ledger_sequence: 123_456_u32,
        };

        let val: Val = real_entry.into_val(&env);
        let our_entry: OurEntry = match OurEntry::try_from_val(&env, &val) {
            Ok(e) => e,
            Err(()) => panic!("Our SpendingEntry mirror failed to decode real OZ SpendingEntry"),
        };

        assert_eq!(our_entry.amount, real_entry.amount);
        assert_eq!(our_entry.ledger_sequence, real_entry.ledger_sequence);
    }
}
