#![cfg(test)]

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token::{Client as TokenClient, StellarAssetClient},
    Address, Env,
};

/// Helper: deploy a mock USDC token and mint to an address.
fn setup_token(env: &Env, admin: &Address, recipient: &Address, amount: i128) -> Address {
    let token_addr = env.register_stellar_asset_contract_v2(admin.clone());
    let token = StellarAssetClient::new(env, &token_addr.address());
    token.mint(recipient, &amount);
    token_addr.address()
}

/// TEST 1 — Happy path: OFW creates transfer, recipient claims successfully.
#[test]
fn test_create_and_claim_transfer() {
    let env = Env::default();
    env.mock_all_auths();

    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let admin = Address::generate(&env);

    let token_addr = setup_token(&env, &admin, &sender, 100_000_000); // 10 USDC

    let contract_id = env.register_contract(None, RemitLinkContract);
    let client = RemitLinkContractClient::new(&env, &contract_id);

    // OFW creates transfer of 10 USDC
    let transfer_id = client.create_transfer(&sender, &recipient, &token_addr, &100_000_000);
    assert_eq!(transfer_id, 1);

    // Recipient claims within window
    client.claim(&transfer_id, &token_addr);

    // Verify recipient received the USDC
    let token = TokenClient::new(&env, &token_addr);
    assert_eq!(token.balance(&recipient), 100_000_000);

    // Verify transfer is marked claimed
    let transfer = client.get_transfer(&transfer_id);
    assert!(transfer.claimed);
}

/// TEST 2 — Edge case: recipient tries to claim an already-claimed transfer (double-spend attempt).
#[test]
#[should_panic(expected = "Transfer already claimed")]
fn test_double_claim_panics() {
    let env = Env::default();
    env.mock_all_auths();

    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let admin = Address::generate(&env);

    let token_addr = setup_token(&env, &admin, &sender, 100_000_000);
    let contract_id = env.register_contract(None, RemitLinkContract);
    let client = RemitLinkContractClient::new(&env, &contract_id);

    let transfer_id = client.create_transfer(&sender, &recipient, &token_addr, &100_000_000);
    client.claim(&transfer_id, &token_addr); // First claim — OK
    client.claim(&transfer_id, &token_addr); // Second claim — should panic
}

/// TEST 3 — State verification: transfer struct is correctly stored after creation.
#[test]
fn test_transfer_state_stored_correctly() {
    let env = Env::default();
    env.mock_all_auths();

    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let admin = Address::generate(&env);
    let amount: i128 = 50_000_000; // 5 USDC

    let token_addr = setup_token(&env, &admin, &sender, amount);
    let contract_id = env.register_contract(None, RemitLinkContract);
    let client = RemitLinkContractClient::new(&env, &contract_id);

    let transfer_id = client.create_transfer(&sender, &recipient, &token_addr, &amount);
    let transfer = client.get_transfer(&transfer_id);

    // Verify all fields stored correctly
    assert_eq!(transfer.sender, sender);
    assert_eq!(transfer.recipient, recipient);
    assert_eq!(transfer.amount, amount);
    assert!(!transfer.claimed);
    // Expiry should be current ledger + 720
    assert_eq!(transfer.expiry_ledger, env.ledger().sequence() + 720);
}

/// TEST 4 — Edge case: sender gets refund after rate-lock window expires.
#[test]
fn test_refund_after_expiry() {
    let env = Env::default();
    env.mock_all_auths();

    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let admin = Address::generate(&env);

    let token_addr = setup_token(&env, &admin, &sender, 100_000_000);
    let contract_id = env.register_contract(None, RemitLinkContract);
    let client = RemitLinkContractClient::new(&env, &contract_id);

    let transfer_id = client.create_transfer(&sender, &recipient, &token_addr, &100_000_000);

    // Advance ledger past the 720-ledger expiry window
    env.ledger().with_mut(|l| {
        l.sequence_number += 721;
    });

    // Sender refunds their USDC
    client.refund(&transfer_id, &token_addr);

    // Verify sender got money back
    let token = TokenClient::new(&env, &token_addr);
    assert_eq!(token.balance(&sender), 100_000_000);
}

/// TEST 5 — Edge case: refund fails if rate-lock window is still active.
#[test]
#[should_panic(expected = "Rate-lock window still active")]
fn test_refund_fails_during_window() {
    let env = Env::default();
    env.mock_all_auths();

    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let admin = Address::generate(&env);

    let token_addr = setup_token(&env, &admin, &sender, 100_000_000);
    let contract_id = env.register_contract(None, RemitLinkContract);
    let client = RemitLinkContractClient::new(&env, &contract_id);

    let transfer_id = client.create_transfer(&sender, &recipient, &token_addr, &100_000_000);

    // Attempt refund immediately — should panic because window is still active
    client.refund(&transfer_id, &token_addr);
}