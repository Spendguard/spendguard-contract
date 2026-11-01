use soroban_sdk::{Address, Env, IntoVal, Symbol};
use crate::types::{
    ContextRule, ContextRuleType, PolicyViewError, SpendingLimitData, SpendingLimitView,
};

/// Queries the OZ smart account contract and its installed
/// `spending_limit` policy contract to return the current spending
/// limit state for the given context rule.
pub fn get_spending_limit_state(
    env: &Env,
    account: &Address,
    context_rule_id: u32,
) -> Result<SpendingLimitView, PolicyViewError> {
    // ------------------------------------------------------------------
    // Step 1: Retrieve the context rule from the smart account contract.
    // ------------------------------------------------------------------
    let context_rule: ContextRule = match env.try_invoke_contract::<ContextRule, soroban_sdk::Error>(
        account,
        &Symbol::new(env, "get_context_rule"),
        (context_rule_id,).into_val(env),
    ) {
        Ok(Ok(rule)) => rule,
        _ => return Err(PolicyViewError::AccountNotFound),
    };

    // ------------------------------------------------------------------
    // Step 2: Only a `CallContract` context can carry a `spending_limit`
    // policy. `Default` and `CreateContract` rules must be rejected here:
    // falling through would query a policy that can never apply to the
    // rule, and report either a bogus view or a misleading
    // `NoSpendingLimitPolicyInstalled`.
    // ------------------------------------------------------------------
    if !matches!(context_rule.context_type, ContextRuleType::CallContract(_)) {
        return Err(PolicyViewError::UnsupportedContextType);
    }

    // ------------------------------------------------------------------
    // Step 3: Iterate over the rule's policy contracts and try to
    // retrieve spending limit data from each.
    // ------------------------------------------------------------------
    let get_lmt_data_sym = Symbol::new(env, "get_spending_limit_data");

    for policy_addr in context_rule.policies.iter() {
        let result = env.try_invoke_contract::<SpendingLimitData, soroban_sdk::Error>(
            &policy_addr,
            &get_lmt_data_sym,
            (context_rule_id, account.clone()).into_val(env),
        );

        if let Ok(Ok(data)) = result {
            return Ok(SpendingLimitView::from(data));
        }
    }

    Err(PolicyViewError::NoSpendingLimitPolicyInstalled)
}
