//! # Policy View Helper — Tests

use soroban_sdk::{Env, Vec};

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
