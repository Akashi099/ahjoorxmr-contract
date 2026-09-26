#![cfg(test)]
use super::*;
use soroban_sdk::token::Client as TokenClient;
use soroban_sdk::token::StellarAssetClient as TokenAdminClient;
use soroban_sdk::{testutils::Address as _, Address, Env, Vec};

fn setup<'a>() -> (Env, AhjoorEscrowContractClient<'a>, Address) {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(AhjoorEscrowContract, ());
    let client = AhjoorEscrowContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin);

    (env, client, admin)
}

/// Before any admin configuration the getter must return the hard-coded
/// default of 48 hours (172 800 seconds).
#[test]
fn test_get_veto_override_window_default() {
    let (_env, client, _admin) = setup();
    assert_eq!(client.get_veto_override_window(), 48 * 60 * 60);
}

/// After set_veto_override_window the getter must reflect the new value.
#[test]
fn test_get_veto_override_window_after_set() {
    let (_env, client, admin) = setup();
    let custom_window: u64 = 72 * 60 * 60; // 72 hours
    client.set_veto_override_window(&admin, &custom_window);
    assert_eq!(client.get_veto_override_window(), custom_window);
}

/// Updating the window a second time must overwrite the previous value.
#[test]
fn test_get_veto_override_window_update() {
    let (_env, client, admin) = setup();
    client.set_veto_override_window(&admin, &(24 * 60 * 60));
    client.set_veto_override_window(&admin, &(96 * 60 * 60));
    assert_eq!(client.get_veto_override_window(), 96 * 60 * 60);
}

/// A non-admin caller must not be able to set the window (auth guard).
#[test]
#[should_panic]
fn test_set_veto_override_window_non_admin_rejected() {
    let (env, client, _admin) = setup();
    let non_admin = Address::generate(&env);
    client.set_veto_override_window(&non_admin, &(24 * 60 * 60));
}

/// window_seconds == 0 must be rejected.
#[test]
#[should_panic]
fn test_set_veto_override_window_zero_rejected() {
    let (_env, client, admin) = setup();
    client.set_veto_override_window(&admin, &0u64);
}
