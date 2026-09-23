//! Tests for hub initialization guard (Issue #495 — C37).
//!
//! Acceptance criteria:
//!   1. Uninitialized calls return the initialization error (`AdminNotSet`).
//!   2. Initialization remains one-time — calling `set_admin` twice does not
//!      reset the initialized flag.
//!   3. Initialized calls retain current behavior — post-init calls work as before.

#![cfg(test)]

use super::*;
use crate::errors::Error;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::{Address, BytesN, Env, String};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Register the contract WITHOUT calling set_admin so the hub is uninitialized.
fn setup_uninitialized() -> (Env, Address) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(Contract, ());
    (env, contract_id)
}

/// Register the contract AND call set_admin so the hub is initialized.
fn setup_initialized() -> (Env, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(Contract, ());
    let client = ContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.set_admin(&admin);
    (env, contract_id, admin)
}

// ---------------------------------------------------------------------------
// Criterion 1 — Uninitialized calls return AdminNotSet
// ---------------------------------------------------------------------------

/// `issue_token` on an uninitialized hub must fail with `AdminNotSet`.
#[test]
fn test_issue_token_uninitialized_fails() {
    let (env, contract_id) = setup_uninitialized();
    let client = ContractClient::new(&env, &contract_id);

    let token_id = BytesN::<32>::random(&env);
    let user = Address::generate(&env);
    let expiry = env.ledger().timestamp() + 86400;

    let result = client.try_issue_token(&token_id, &user, &expiry);
    assert!(
        result.is_err(),
        "issue_token on uninitialized hub must return error"
    );
}

/// `create_subscription` on an uninitialized hub must fail.
#[test]
fn test_create_subscription_uninitialized_fails() {
    let (env, contract_id) = setup_uninitialized();
    let client = ContractClient::new(&env, &contract_id);

    let user = Address::generate(&env);
    let payment_token = Address::generate(&env);

    let result = client.try_create_subscription(
        &String::from_str(&env, "sub_init_test"),
        &user,
        &payment_token,
        &100i128,
        &86400u64,
    );
    assert!(
        result.is_err(),
        "create_subscription on uninitialized hub must return error"
    );
}

/// `stake_tokens` on an uninitialized hub must fail.
#[test]
fn test_stake_tokens_uninitialized_fails() {
    let (env, contract_id) = setup_uninitialized();
    let client = ContractClient::new(&env, &contract_id);

    let staker = Address::generate(&env);
    let result = client.try_stake_tokens(
        &staker,
        &String::from_str(&env, "tier_one"),
        &1000i128,
    );
    assert!(
        result.is_err(),
        "stake_tokens on uninitialized hub must return error"
    );
}

/// `is_hub_initialized` returns false before `set_admin` is called.
#[test]
fn test_is_hub_initialized_false_before_set_admin() {
    let (env, contract_id) = setup_uninitialized();
    let client = ContractClient::new(&env, &contract_id);

    let initialized = client.is_hub_initialized();
    assert!(
        !initialized,
        "hub must report not-initialized before set_admin is called"
    );
}

// ---------------------------------------------------------------------------
// Criterion 2 — Initialization is one-time; re-calling set_admin does not reset
// ---------------------------------------------------------------------------

/// After `set_admin`, `is_hub_initialized` returns `true`.
#[test]
fn test_is_hub_initialized_true_after_set_admin() {
    let (env, contract_id, _admin) = setup_initialized();
    let client = ContractClient::new(&env, &contract_id);

    assert!(
        client.is_hub_initialized(),
        "hub must report initialized after set_admin"
    );
}

/// Calling `set_admin` a second time (admin re-sets themselves) does not
/// unset the initialized flag.
#[test]
fn test_second_set_admin_does_not_reset_initialized_flag() {
    let (env, contract_id, admin) = setup_initialized();
    let client = ContractClient::new(&env, &contract_id);

    // Re-set the admin (allowed because current_admin == new_admin).
    client.set_admin(&admin);

    assert!(
        client.is_hub_initialized(),
        "initialized flag must persist after second set_admin call"
    );
}

// ---------------------------------------------------------------------------
// Criterion 3 — Initialized calls retain current behavior
// ---------------------------------------------------------------------------

/// `issue_token` succeeds on an initialized hub with valid inputs.
#[test]
fn test_issue_token_initialized_succeeds() {
    let (env, contract_id, _admin) = setup_initialized();
    let client = ContractClient::new(&env, &contract_id);

    let token_id = BytesN::<32>::random(&env);
    let user = Address::generate(&env);
    let expiry = env.ledger().timestamp() + 86400;

    let result = client.try_issue_token(&token_id, &user, &expiry);
    assert!(
        result.is_ok(),
        "issue_token must succeed on an initialized hub"
    );
}

/// `create_subscription` succeeds on an initialized hub.
#[test]
fn test_create_subscription_initialized_succeeds() {
    let (env, contract_id, _admin) = setup_initialized();
    let client = ContractClient::new(&env, &contract_id);

    let user = Address::generate(&env);
    let payment_token = Address::generate(&env);

    // Set the USDC contract so validate_payment does not fail.
    client.set_usdc_contract(&_admin, &payment_token);

    let result = client.try_create_subscription(
        &String::from_str(&env, "init_sub"),
        &user,
        &payment_token,
        &100i128,
        &86400u64,
    );
    assert!(
        result.is_ok(),
        "create_subscription must succeed on an initialized hub"
    );
}

/// Multiple tokens can be issued on an initialized hub (no double-init side effect).
#[test]
fn test_multiple_operations_after_init_succeed() {
    let (env, contract_id, _admin) = setup_initialized();
    let client = ContractClient::new(&env, &contract_id);

    for i in 0u32..3 {
        let token_id = BytesN::<32>::random(&env);
        let user = Address::generate(&env);
        let expiry = env.ledger().timestamp() + 86400 + u64::from(i);
        client.issue_token(&token_id, &user, &expiry);
    }

    // Hub still reports initialized.
    assert!(client.is_hub_initialized());
}
