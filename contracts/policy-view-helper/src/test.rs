//! # Policy View Helper — Tests

use soroban_sdk::{
    contract, contractimpl,
    testutils::{Address as _, Ledger as _},
    Address, BytesN, Env, String, Symbol, Vec,
};

use crate::types::{
    ContextRule, ContextRuleType, PolicyViewError,
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
        signers: Vec::new(env),
        signer_ids: Vec::new(env),
        policies: policy_addrs,
        policy_ids: Vec::new(env),
        valid_until: None,
    }
}

// ──────────────────────────────────────────────────────────────────────────
// Error assertions
// ──────────────────────────────────────────────────────────────────────────

/// The contract error surfaced by a `try_get_spending_limit_state` call.
///
/// The generated `try_` client nests four outcomes, and only one of them is a
/// tested failure mode:
///
/// | Shape | Meaning |
/// |---|---|
/// | `Ok(Ok(view))` | the helper returned a view |
/// | `Ok(Err(conv))` | the helper returned, but the client could not decode it |
/// | `Err(Ok(err))` | the helper ran and returned this `PolicyViewError` |
/// | `Err(Err(inv))` | the call never produced a contract error (host failure) |
///
/// Each other branch panics and names what actually happened. `assert!(x.is_err())`
/// accepted all four, and any of the three error variants for any failure mode —
/// which is how #1 stayed green.
fn policy_view_error<T: core::fmt::Debug>(
    result: Result<
        Result<T, soroban_sdk::ConversionError>,
        Result<PolicyViewError, soroban_sdk::InvokeError>,
    >,
) -> PolicyViewError {
    match result {
        Ok(Ok(view)) => panic!("expected a PolicyViewError, got Ok({view:?})"),
        Ok(Err(conversion)) => {
            panic!("helper returned data the client could not decode: {conversion:?}")
        }
        Err(Ok(contract_error)) => contract_error,
        Err(Err(invoke_error)) => {
            panic!("expected a PolicyViewError, got invoke error {invoke_error:?}")
        }
    }
}

// ──────────────────────────────────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────────────────────────────────

/// `get_version` always returns 2.
///
/// Bumped from 1 by #5: `get_spending_limit_state` can now answer
/// `MultipleSpendingLimitPolicies`, and
/// `get_spending_limit_policy_views` was added. The indexer must read this
/// before assuming a single view describes the account's limit.
#[test]
fn test_get_version() {
    let env = Env::default();
    let contract_id = env.register(crate::PolicyViewHelper, ());
    let client = crate::PolicyViewHelperClient::new(&env, &contract_id);

    assert_eq!(client.get_version(), 2);
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

    let view: SpendingLimitView = client.get_spending_limit_state(&account_addr, &1);

    assert_eq!(view.cap, 10_000_000_000);
    assert_eq!(view.spent_in_window, 2_500_000_000);
    assert_eq!(view.window_ledgers, 17280);
    assert_eq!(view.window_started_ledger, 0);
}

/// Negative case: the context rule exists but carries no policies at all,
/// so nothing can answer `get_spending_limit_data`.
#[test]
fn test_get_spending_limit_state_no_policy() {
    let env = Env::default();
    let rule = default_context_rule(&env, Vec::new(&env));
    let account_addr = deploy_mock_smart_account(&env, &rule);

    let contract_id = env.register(crate::PolicyViewHelper, ());
    let client = crate::PolicyViewHelperClient::new(&env, &contract_id);

    let result = client.try_get_spending_limit_state(&account_addr, &1);
    assert_eq!(
        policy_view_error(result),
        PolicyViewError::NoSpendingLimitPolicyInstalled,
    );
}

/// Negative case: policies exist but none exposes
/// `get_spending_limit_data`, so the policy loop exhausts without a reply.
#[test]
fn test_get_spending_limit_state_no_matching_policy() {
    let env = Env::default();
    let other_policy = deploy_mock_other_policy(&env);
    let rule = default_context_rule(&env, Vec::from_array(&env, [other_policy]));
    let account_addr = deploy_mock_smart_account(&env, &rule);

    let contract_id = env.register(crate::PolicyViewHelper, ());
    let client = crate::PolicyViewHelperClient::new(&env, &contract_id);

    let result = client.try_get_spending_limit_state(&account_addr, &1);
    assert_eq!(
        policy_view_error(result),
        PolicyViewError::NoSpendingLimitPolicyInstalled,
    );
}

/// Negative case for `AccountNotFound`: the address handed in as the smart
/// account is a real contract that does not implement `get_context_rule`, so
/// step 1 fails before any policy is consulted. Nothing else in the suite
/// produced this variant — the error existed untested.
#[test]
fn test_get_spending_limit_state_account_not_found() {
    let env = Env::default();
    let not_an_account = deploy_mock_other_policy(&env);

    let contract_id = env.register(crate::PolicyViewHelper, ());
    let client = crate::PolicyViewHelperClient::new(&env, &contract_id);

    let result = client.try_get_spending_limit_state(&not_an_account, &1);
    assert_eq!(
        policy_view_error(result),
        PolicyViewError::AccountNotFound,
    );
}

/// Negative case: context rule type is `CreateContract`, which can never
/// carry a spending limit, so the guard answers `UnsupportedContextType`
/// rather than reaching the policy loop.
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
        signers: Vec::new(&env),
        signer_ids: Vec::new(&env),
        policies: Vec::from_array(&env, [policy_addr]),
        policy_ids: Vec::new(&env),
        valid_until: None,
    };
    let account_addr = deploy_mock_smart_account(&env, &create_contract_rule);

    let contract_id = env.register(crate::PolicyViewHelper, ());
    let client = crate::PolicyViewHelperClient::new(&env, &contract_id);

    let result = client.try_get_spending_limit_state(&account_addr, &2);
    assert_eq!(
        policy_view_error(result),
        PolicyViewError::UnsupportedContextType,
    );
}

/// Regression: a `Default` context rule with a `spending_limit` policy
/// attached must still be rejected.
///
/// The guard used to read `!is_call_contract && is_create_contract`, so a
/// `Default` rule satisfied neither half, slipped past the check, and the
/// policy loop happily returned the attached policy's data as if the rule
/// were a supported transfer context.
#[test]
fn test_get_spending_limit_state_default_context_is_unsupported() {
    let env = Env::default();
    let policy_addr =
        deploy_mock_spending_limit_policy(&env, &default_spending_limit_data(&env));

    let default_rule = ContextRule {
        id: 3,
        context_type: ContextRuleType::Default,
        name: String::from_str(&env, "default-rule"),
        signers: Vec::new(&env),
        signer_ids: Vec::new(&env),
        policies: Vec::from_array(&env, [policy_addr]),
        policy_ids: Vec::new(&env),
        valid_until: None,
    };
    let account_addr = deploy_mock_smart_account(&env, &default_rule);

    let contract_id = env.register(crate::PolicyViewHelper, ());
    let client = crate::PolicyViewHelperClient::new(&env, &contract_id);

    let result = client.try_get_spending_limit_state(&account_addr, &3);
    assert_eq!(
        policy_view_error(result),
        PolicyViewError::UnsupportedContextType,
        "a Default context rule must be rejected by the context guard, not by \
         the policy loop"
    );
}

/// Edge case: spending history has entries → `window_started_ledger`
/// matches the earliest entry's ledger sequence.
#[test]
fn test_get_spending_limit_state_with_history() {
    let env = Env::default();
    // Set a realistic ledger sequence so the relative offsets below don't
    // underflow the test environment's default (very low) sequence.
    env.ledger().set_sequence_number(200_000);
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

    let view: SpendingLimitView = client.get_spending_limit_state(&account_addr, &1);

    assert_eq!(view.spent_in_window, 2_500_000_000);
    // window_started_ledger should equal the earliest entry's ledger sequence
    assert_eq!(view.window_started_ledger, current_ledger - 1000);
}

// ──────────────────────────────────────────────────────────────────────────
// More than one spending_limit policy on a rule
//
// OZ lets a context rule carry several policies, and two `spending_limit`
// policies with different caps is a legal configuration. The helper used to
// return the first policy that answered, which is the account's storage order
// rather than anything about the limits: with caps of 1 000 and 100 000 the
// dashboard could display the loose one at 40 % while the tight one was the
// limit actually binding the next spend.
//
// `get_spending_limit_policy_views` reports every responding policy, and
// `get_spending_limit_state` refuses to pick.
// ──────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod multiple_policies {
    use super::{
        default_context_rule, default_spending_limit_data, deploy_mock_other_policy,
        deploy_mock_smart_account, deploy_mock_spending_limit_policy, policy_view_error,
        SpendingLimitData,
    };
    use crate::types::PolicyViewError;
    use soroban_sdk::{Address, Env, Vec};

    fn data_with_cap(env: &Env, cap: i128, spent: i128) -> SpendingLimitData {
        let mut data = default_spending_limit_data(env);
        data.spending_limit = cap;
        data.cached_total_spent = spent;
        data
    }

    /// The two caps are deliberately ordered loose-first in one test and
    /// tight-first in the other: whichever the account stores first, the single
    /// view must refuse instead of returning it.
    fn rule_with_two_limits(
        env: &Env,
        first_cap: i128,
        second_cap: i128,
    ) -> (Address, Vec<Address>, Address, Address) {
        let loose = deploy_mock_spending_limit_policy(env, &data_with_cap(env, first_cap, 2_500_000_000));
        let tight = deploy_mock_spending_limit_policy(
            env,
            &data_with_cap(env, second_cap, 250_000_000),
        );
        let policies = Vec::from_array(env, [loose.clone(), tight.clone()]);
        let rule = default_context_rule(env, policies.clone());
        let account = deploy_mock_smart_account(env, &rule);
        (account, policies, loose, tight)
    }

    fn helper<'env>(env: &'env Env) -> crate::PolicyViewHelperClient<'env> {
        let id = env.register(crate::PolicyViewHelper, ());
        crate::PolicyViewHelperClient::new(env, &id)
    }

    #[test]
    fn single_policy_still_returns_one_view() {
        let env = Env::default();
        let policy = deploy_mock_spending_limit_policy(&env, &default_spending_limit_data(&env));
        let rule = default_context_rule(&env, Vec::from_array(&env, [policy.clone()]));
        let account = deploy_mock_smart_account(&env, &rule);

        let views = helper(&env).get_spending_limit_policy_views(&account, &1);

        assert_eq!(views.len(), 1);
        assert_eq!(views.get(0).unwrap().policy, policy);
        assert_eq!(views.get(0).unwrap().view.cap, 10_000_000_000);
    }

    #[test]
    fn every_responding_policy_is_reported_with_its_own_cap() {
        let env = Env::default();
        let (account, policies, loose, tight) = rule_with_two_limits(&env, 10_000_000_000, 1_000_000_000);

        let views = helper(&env).get_spending_limit_policy_views(&account, &1);

        assert_eq!(views.len(), 2, "both limits must be visible");
        // The address is what makes the view actionable; without it a caller
        // cannot tell which policy produced which cap.
        assert_eq!(views.get(0).unwrap().policy, loose);
        assert_eq!(views.get(0).unwrap().view.cap, 10_000_000_000);
        assert_eq!(views.get(0).unwrap().view.spent_in_window, 2_500_000_000);
        assert_eq!(views.get(1).unwrap().policy, tight);
        assert_eq!(views.get(1).unwrap().view.cap, 1_000_000_000);
        assert_eq!(views.get(1).unwrap().view.spent_in_window, 250_000_000);

        // Both entries keep the policy's own window, not a merged guess.
        for entry in views.iter() {
            assert_eq!(entry.view.window_ledgers, 17_280);
        }
        assert_eq!(policies.len(), 2);
    }

    #[test]
    fn the_single_view_refuses_instead_of_picking_when_two_answer() {
        let env = Env::default();
        let (account, _, _, _) = rule_with_two_limits(&env, 10_000_000_000, 1_000_000_000);

        let result = helper(&env).try_get_spending_limit_state(&account, &1);

        assert_eq!(
            policy_view_error(result),
            PolicyViewError::MultipleSpendingLimitPolicies,
            "returning either policy is the bug; the helper must not choose"
        );
    }

    /// The old code returned `policies[first answering]`. Reversing the rule's
    /// vector therefore flipped the reported cap. The refusal has to be
    /// order-independent, so the same expectation holds on the reversed rule.
    #[test]
    fn refusing_does_not_depend_on_the_policies_vector_order() {
        let env = Env::default();
        let tight = deploy_mock_spending_limit_policy(&env, &data_with_cap(&env, 1_000_000_000, 250_000_000));
        let loose = deploy_mock_spending_limit_policy(
            &env,
            &data_with_cap(&env, 10_000_000_000, 2_500_000_000),
        );
        let rule = default_context_rule(&env, Vec::from_array(&env, [tight.clone(), loose.clone()]));
        let account = deploy_mock_smart_account(&env, &rule);

        let client = helper(&env);

        assert_eq!(
            policy_view_error(client.try_get_spending_limit_state(&account, &1)),
            PolicyViewError::MultipleSpendingLimitPolicies,
        );

        let views = client.get_spending_limit_policy_views(&account, &1);
        // Enumeration still follows the rule's own order — documented, not
        // meaningful — and each entry is labelled, so the order cannot be
        // mistaken for a ranking.
        assert_eq!(views.get(0).unwrap().policy, tight);
        assert_eq!(views.get(1).unwrap().policy, loose);
    }

    /// A rule carrying one spending-limit policy and one unrelated policy is
    /// the common real configuration; the unrelated one must not force a
    /// refusal.
    #[test]
    fn non_responding_policies_do_not_count_as_extra_limits() {
        let env = Env::default();
        let limit = deploy_mock_spending_limit_policy(&env, &default_spending_limit_data(&env));
        let other = deploy_mock_other_policy(&env);
        let rule = default_context_rule(&env, Vec::from_array(&env, [other, limit.clone()]));
        let account = deploy_mock_smart_account(&env, &rule);

        let client = helper(&env);
        let view = client.get_spending_limit_state(&account, &1);

        assert_eq!(view.cap, 10_000_000_000);
        assert_eq!(
            client.get_spending_limit_policy_views(&account, &1).get(0).unwrap().policy,
            limit
        );
    }

    #[test]
    fn the_multi_view_call_keeps_the_same_guards_as_the_single_one() {
        let env = Env::default();
        let client = helper(&env);

        // Step 1: not a smart account at all.
        let not_an_account = deploy_mock_other_policy(&env);
        assert_eq!(
            policy_view_error(client.try_get_spending_limit_policy_views(&not_an_account, &1)),
            PolicyViewError::AccountNotFound,
        );

        // Step 2: a `Default` rule with a real limit attached is still rejected
        // rather than enumerated.
        let limit = deploy_mock_spending_limit_policy(&env, &default_spending_limit_data(&env));
        let default_rule = crate::types::ContextRule {
            id: 3,
            context_type: crate::types::ContextRuleType::Default,
            name: soroban_sdk::String::from_str(&env, "default-rule"),
            signers: Vec::new(&env),
            signer_ids: Vec::new(&env),
            policies: Vec::from_array(&env, [limit]),
            policy_ids: Vec::new(&env),
            valid_until: None,
        };
        let account = deploy_mock_smart_account(&env, &default_rule);
        assert_eq!(
            policy_view_error(client.try_get_spending_limit_policy_views(&account, &3)),
            PolicyViewError::UnsupportedContextType,
        );

        // Step 3: nothing answered.
        let empty_rule = default_context_rule(&env, Vec::new(&env));
        let account = deploy_mock_smart_account(&env, &empty_rule);
        assert_eq!(
            policy_view_error(client.try_get_spending_limit_policy_views(&account, &1)),
            PolicyViewError::NoSpendingLimitPolicyInstalled,
        );
    }
}

// ──────────────────────────────────────────────────────────────────────────
// OZ Serialization Compatibility Tests
//
// These tests verify that our mirror types (defined in `types.rs`) can
// correctly decode data serialized by the real `stellar-accounts` v0.7.2
// types.  This catches field-order or memory-layout mismatches between
// the OZ `#[contracttype]` structs and our local mirrors.
//
// Because `stellar-accounts` is a dev-dependency, we import the real
// types directly and round-trip through `Env`-based serialization.
// ──────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod oz_serialization_compat {
    use soroban_sdk::{testutils::Address as _, Address, Env, IntoVal, TryFromVal, Val, Vec};

    // Import the REAL stellar-accounts v0.7.2 types, aliased to avoid name
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
            spending_history: Vec::from_array(&env, [real_entry.clone()]),
            cached_total_spent: 1_000_000_000i128,
        };

        // Serialize using the real OZ type
        let val: Val = real_data.into_val(&env);

        // Deserialize using our mirror type
        let our_data: OurData = match OurData::try_from_val(&env, &val) {
            Ok(d) => d,
            Err(_) => panic!("Our SpendingLimitData mirror failed to decode real OZ data"),
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
            signers: Vec::new(&env),
            signer_ids: Vec::new(&env),
            policies: Vec::from_array(&env, [policy_addr.clone()]),
            policy_ids: Vec::new(&env),
            valid_until: Some(200_000_u32),
        };

        // Serialize using the real OZ type
        let val: Val = real_rule.into_val(&env);

        // Deserialize using our mirror type
        let our_rule: OurContextRule = match OurContextRule::try_from_val(&env, &val) {
            Ok(r) => r,
            Err(_) => panic!("Our ContextRule mirror failed to decode real OZ ContextRule"),
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
            Err(_) => panic!("Our SpendingEntry mirror failed to decode real OZ SpendingEntry"),
        };

        assert_eq!(our_entry.amount, real_entry.amount);
        assert_eq!(our_entry.ledger_sequence, real_entry.ledger_sequence);
    }
}

// ──────────────────────────────────────────────────────────────────────────
// window_started_ledger: ordering
//
// `SpendingLimitView::from` used to read `spending_history.first()`, which is
// only the earliest entry if the policy keeps the vector sorted oldest-first.
// `stellar-accounts` 0.7.2 does — `enforce` appends with `push_back` and
// `cleanup_old_entries` prunes from the front, breaking at the first entry
// still inside the window (a loop that is only correct on a sorted vector) —
// but that is a detail of someone else's storage layout, and getting it wrong
// is asymmetric: a flipped order would report the newest spend as the window
// start, making a limit about to expire look nearly reset.
//
// The tests below take the minimum instead, and prove both halves: the view is
// order-independent, and the real dependency's ordering is what we think it is.
// ──────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod window_start_ordering {
    use super::{
        default_context_rule, default_spending_limit_data, deploy_mock_smart_account,
        deploy_mock_spending_limit_policy, SpendingEntry,
    };
    use soroban_sdk::{
        auth::{Context, ContractContext},
        contract, contractimpl, symbol_short,
        testutils::{Address as _, Ledger as _},
        Address, Env, IntoVal, String, Vec,
    };

    use stellar_accounts::policies::spending_limit::{
        self as oz, SpendingLimitAccountParams, SpendingLimitData as OzData,
    };
    use stellar_accounts::smart_account::{
        ContextRule as OzContextRule, ContextRuleType as OzContextRuleType, Signer as OzSigner,
    };

    /// The real OZ spending-limit policy state machine, hosted on a contract so
    /// the helper can reach it through the same cross-contract call the indexer
    /// makes. Storage belongs to this contract, as it would to a deployed
    /// policy — nothing here is a hand-built vector.
    #[contract]
    pub struct RealSpendingLimitPolicy;

    #[contractimpl]
    impl RealSpendingLimitPolicy {
        pub fn get_spending_limit_data(
            env: Env,
            context_rule_id: u32,
            smart_account: Address,
        ) -> OzData {
            oz::get_spending_limit_data(&env, context_rule_id, &smart_account)
        }
    }

    fn real_rule(env: &Env) -> OzContextRule {
        OzContextRule {
            id: 1,
            context_type: OzContextRuleType::CallContract(Address::generate(env)),
            name: String::from_str(env, "real-rule"),
            signers: Vec::new(env),
            signer_ids: Vec::new(env),
            policies: Vec::new(env),
            policy_ids: Vec::new(env),
            valid_until: None,
        }
    }

    fn transfer_context(env: &Env, amount: i128) -> Context {
        let mut args = Vec::new(env);
        args.push_back(Address::generate(env).into_val(env));
        args.push_back(Address::generate(env).into_val(env));
        args.push_back(amount.into_val(env));
        Context::Contract(ContractContext {
            contract: Address::generate(env),
            fn_name: symbol_short!("transfer"),
            args,
        })
    }

    /// Reads the window start from a policy that is the real OZ code: two
    /// settlements 70 ledgers apart, and the helper's view must name the older
    /// one — with cap, window and spent total decoded through our mirror types.
    #[test]
    fn window_started_ledger_comes_from_the_real_policy_history() {
        let env = Env::default();
        env.mock_all_auths();
        env.ledger().set_sequence_number(200_000);

        let policy = env.register(RealSpendingLimitPolicy, ());

        // The account the helper will query with is also the account the policy
        // is written under: OZ keys its state by
        // `AccountContext(smart_account, context_rule_id)`, so installing under a
        // different address would make the helper read an empty history and prove
        // nothing.
        let our_rule = default_context_rule(&env, Vec::from_array(&env, [policy.clone()]));
        let account = deploy_mock_smart_account(&env, &our_rule);

        let rule = real_rule(&env);
        let signers = Vec::from_array(&env, [OzSigner::Delegated(Address::generate(&env))]);
        let params = SpendingLimitAccountParams {
            spending_limit: 10_000_000,
            period_ledgers: 1_000,
        };

        env.as_contract(&policy, || {
            oz::install(&env, &params, &rule, &account);
        });

        env.ledger().set_sequence_number(200_050);
        env.as_contract(&policy, || {
            oz::enforce(
                &env,
                &transfer_context(&env, 1_000_000),
                &signers,
                &rule,
                &account,
            );
        });

        env.ledger().set_sequence_number(200_120);
        env.as_contract(&policy, || {
            oz::enforce(
                &env,
                &transfer_context(&env, 500_000),
                &signers,
                &rule,
                &account,
            );
        });

        // The ordering claim, taken from the dependency rather than assumed.
        let raw = env.as_contract(&policy, || oz::get_spending_limit_data(&env, 1, &account));
        assert_eq!(raw.spending_history.len(), 2);
        assert_eq!(
            raw.spending_history.get(0).unwrap().ledger_sequence,
            200_050,
            "stellar-accounts 0.7.2 retains the oldest settlement first"
        );
        assert_eq!(
            raw.spending_history.get(1).unwrap().ledger_sequence,
            200_120
        );

        let helper = env.register(crate::PolicyViewHelper, ());
        let view = crate::PolicyViewHelperClient::new(&env, &helper)
            .get_spending_limit_state(&account, &1);

        assert_eq!(view.cap, 10_000_000);
        assert_eq!(view.window_ledgers, 1_000);
        assert_eq!(view.spent_in_window, 1_500_000);
        assert_eq!(view.window_started_ledger, 200_050);
    }

    /// The same read against a history in an order the current dependency never
    /// produces. `first()` would answer 199_500 — the newest-but-one spend, a
    /// window that looks like it is about to reopen when it is about to close.
    #[test]
    fn window_started_ledger_ignores_history_order() {
        let env = Env::default();
        env.ledger().set_sequence_number(200_000);

        let mut data = default_spending_limit_data(&env);
        data.spending_history = Vec::from_array(
            &env,
            [
                SpendingEntry {
                    amount: 1_000_000,
                    ledger_sequence: 199_500,
                },
                SpendingEntry {
                    amount: 1_000_000,
                    ledger_sequence: 197_000,
                },
                SpendingEntry {
                    amount: 1_000_000,
                    ledger_sequence: 199_050,
                },
            ],
        );

        let policy = deploy_mock_spending_limit_policy(&env, &data);
        let rule = default_context_rule(&env, Vec::from_array(&env, [policy]));
        let account = deploy_mock_smart_account(&env, &rule);

        let helper = env.register(crate::PolicyViewHelper, ());
        let view = crate::PolicyViewHelperClient::new(&env, &helper)
            .get_spending_limit_state(&account, &1);

        assert_eq!(view.window_started_ledger, 197_000);
    }

    /// No history at all still means "window start unknown", encoded as 0.
    #[test]
    fn empty_history_keeps_window_started_ledger_at_zero() {
        let env = Env::default();
        let data = default_spending_limit_data(&env);

        let policy = deploy_mock_spending_limit_policy(&env, &data);
        let rule = default_context_rule(&env, Vec::from_array(&env, [policy]));
        let account = deploy_mock_smart_account(&env, &rule);

        let helper = env.register(crate::PolicyViewHelper, ());
        let view = crate::PolicyViewHelperClient::new(&env, &helper)
            .get_spending_limit_state(&account, &1);

        assert_eq!(view.window_started_ledger, 0);
    }
}
