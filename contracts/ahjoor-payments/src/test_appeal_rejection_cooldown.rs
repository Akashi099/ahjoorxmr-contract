#![cfg(test)]
extern crate alloc;
use super::*;
use soroban_sdk::{testutils::Address as _, Address, Env};

fn setup<'a>() -> (Env, AhjoorPaymentsContractClient<'a>, Address) {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(AhjoorPaymentsContract, ());
    let client = AhjoorPaymentsContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let fee_recipient = Address::generate(&env);
    client.initialize(&admin, &fee_recipient, &0);

    (env, client, admin)
}

/// Before any admin configuration the getter must return the hard-coded
/// default of 30 days (2 592 000 seconds).
#[test]
fn test_get_appeal_rejection_cooldown_default() {
    let (_env, client, _admin) = setup();
    assert_eq!(client.get_appeal_rejection_cooldown(), 30 * 24 * 60 * 60);
}

/// After set_appeal_rejection_cooldown the getter must reflect the new value.
#[test]
fn test_get_appeal_rejection_cooldown_after_set() {
    let (_env, client, admin) = setup();
    let custom_cooldown: u64 = 7 * 24 * 60 * 60; // 7 days
    client.set_appeal_rejection_cooldown(&admin, &custom_cooldown);
    assert_eq!(client.get_appeal_rejection_cooldown(), custom_cooldown);
}

/// Updating the cooldown a second time must overwrite the previous value.
#[test]
fn test_get_appeal_rejection_cooldown_update() {
    let (_env, client, admin) = setup();
    client.set_appeal_rejection_cooldown(&admin, &(7 * 24 * 60 * 60));
    client.set_appeal_rejection_cooldown(&admin, &(14 * 24 * 60 * 60));
    assert_eq!(client.get_appeal_rejection_cooldown(), 14 * 24 * 60 * 60);
}

/// A non-admin caller must not be able to configure the cooldown (auth guard).
#[test]
#[should_panic]
fn test_set_appeal_rejection_cooldown_non_admin_rejected() {
    let (env, client, _admin) = setup();
    let non_admin = Address::generate(&env);
    client.set_appeal_rejection_cooldown(&non_admin, &(7 * 24 * 60 * 60));
}
