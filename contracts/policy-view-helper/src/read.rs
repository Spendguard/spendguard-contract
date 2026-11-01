use soroban_sdk::{Address, Env, IntoVal, Symbol, Vec};
use crate::types::{
    ContextRule, ContextRuleType, PolicyViewError, SpendingLimitData, SpendingLimitPolicyView,
    SpendingLimitView,
};

/// Runs steps 1 and 2 shared by both read functions: fetch the context rule
/// from the smart account and confirm the context can carry a spending limit
/// at all.
fn resolve_context_rule(
    env: &Env,
    account: &Address,
    context_rule_id: u32,
) -> Result<ContextRule, PolicyViewError> {
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

    Ok(context_rule)
}

/// Step 3: ask every policy attached to the rule for its spending-limit data
/// and keep the ones that answer.
///
/// Returns in `context_rule.policies` order. That order is the smart account's
/// own storage detail and means nothing semantically, which is exactly why no
/// caller may be handed "the" first answer as if it were the limit — see
/// `get_spending_limit_state`.
fn collect_spending_limit_policies(
    env: &Env,
    context_rule: &ContextRule,
    account: &Address,
    context_rule_id: u32,
) -> Vec<SpendingLimitPolicyView> {
    let get_lmt_data_sym = Symbol::new(env, "get_spending_limit_data");
    let mut responding = Vec::new(env);

    for policy_addr in context_rule.policies.iter() {
        let result = env.try_invoke_contract::<SpendingLimitData, soroban_sdk::Error>(
            &policy_addr,
            &get_lmt_data_sym,
            (context_rule_id, account.clone()).into_val(env),
        );

        if let Ok(Ok(data)) = result {
            responding.push_back(SpendingLimitPolicyView {
                policy: policy_addr.clone(),
                view: SpendingLimitView::from(data),
            });
        }
    }

    responding
}

/// Every `spending_limit` policy on the rule, each labelled with the contract
/// that reports it.
///
/// This is the call to use when a rule may carry more than one policy: an OZ
/// smart account can install several `spending_limit` policies on the same
/// context rule with different caps, and the effective limit is the binding
/// one. Deciding which of them binds — smallest remaining headroom, smallest
/// cap, shortest window — depends on what the caller is asking, and the two
/// candidates can disagree. The helper therefore reports all of them rather
/// than choosing.
///
/// # Errors
///
/// * [`PolicyViewError::NoSpendingLimitPolicyInstalled`] — nothing on the rule
///   answered.
/// * [`PolicyViewError::AccountNotFound`] — step 1.
/// * [`PolicyViewError::UnsupportedContextType`] — step 2.
pub fn get_spending_limit_policy_views(
    env: &Env,
    account: &Address,
    context_rule_id: u32,
) -> Result<Vec<SpendingLimitPolicyView>, PolicyViewError> {
    let context_rule = resolve_context_rule(env, account, context_rule_id)?;

    let responding = collect_spending_limit_policies(env, &context_rule, account, context_rule_id);
    if responding.is_empty() {
        return Err(PolicyViewError::NoSpendingLimitPolicyInstalled);
    }
    Ok(responding)
}

/// The single spending-limit view for a rule that carries exactly one
/// `spending_limit` policy.
///
/// # Errors
///
/// * [`PolicyViewError::MultipleSpendingLimitPolicies`] — the rule carries more
///   than one. The previous implementation returned the first policy that
///   answered, which is arbitrary: two policies with caps 100 and 1 000 report
///   whichever happens to sit first in the account's storage vector, so the
///   dashboard can show 40 % utilisation against the loose limit while a spend
///   is blocked by the tight one. Refusing is the only honest answer the helper
///   can give without knowing the caller's intent; read
///   [`get_spending_limit_policy_views`] and decide there.
/// * [`PolicyViewError::NoSpendingLimitPolicyInstalled`],
///   [`PolicyViewError::AccountNotFound`],
///   [`PolicyViewError::UnsupportedContextType`] — as above.
pub fn get_spending_limit_state(
    env: &Env,
    account: &Address,
    context_rule_id: u32,
) -> Result<SpendingLimitView, PolicyViewError> {
    let responding = get_spending_limit_policy_views(env, account, context_rule_id)?;

    if responding.len() > 1 {
        return Err(PolicyViewError::MultipleSpendingLimitPolicies);
    }

    // `get_spending_limit_policy_views` guarantees at least one entry.
    Ok(responding.get(0).unwrap().view)
}
